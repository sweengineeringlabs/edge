//! [`GrpcOutbound`] impl for [`GrpcRetryClient`].
//!
//! The retry loop:
//! 1. Issue the call with a per-attempt deadline trimmed to fit
//!    inside the caller's remaining budget.
//! 2. On `Ok` → return immediately.
//! 3. On `Err` → classify; terminal → return; retryable → sleep
//!    backoff, advance attempt counter, retry.
//! 4. Stop when:
//!    - attempts reach `config.max_attempts`, OR
//!    - the elapsed wall-time + next backoff would exceed the
//!      caller's deadline (the deadline is the retry budget).
//!
//! Streaming calls (`call_stream`) are NOT retried — gRPC client
//! streaming has no idempotency contract, and re-issuing a
//! half-consumed stream is unsafe.  Streaming delegates straight
//! through to the inner client.
//!
//! Health checks (`health_check`) are also delegated directly —
//! a probe failing should be visible to the caller, not papered
//! over with retries.

use std::time::Instant;

use futures::future::BoxFuture;
use swe_edge_egress_grpc::{
    GrpcMessageStream, GrpcMetadata, GrpcOutbound, GrpcOutboundError, GrpcOutboundResult,
    GrpcRequest, GrpcResponse,
};
use tracing::{debug, trace, warn};

use crate::api::retry_client::GrpcRetryClient;
use crate::api::retry_policy::{classify, RetryDecision};
use crate::core::backoff::{next_backoff, JitterRng};

impl<T: GrpcOutbound + Send + Sync + 'static> GrpcOutbound for GrpcRetryClient<T> {
    fn call_unary(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcOutboundResult<GrpcResponse>> {
        Box::pin(self.run_with_retry(request))
    }

    fn call_stream(
        &self,
        method:   String,
        metadata: GrpcMetadata,
        messages: GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcOutboundResult<GrpcMessageStream>> {
        // Streaming: pass through.  Re-issuing a half-consumed
        // request stream isn't safe in general.
        self.inner.call_stream(method, metadata, messages)
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcOutboundResult<()>> {
        self.inner.health_check()
    }
}

impl<T: GrpcOutbound + Send + Sync + 'static> GrpcRetryClient<T> {
    async fn run_with_retry(
        &self,
        request: GrpcRequest,
    ) -> GrpcOutboundResult<GrpcResponse> {
        let started        = Instant::now();
        let total_budget   = request.deadline;
        let max_attempts   = self.config.max_attempts;
        let mut rng        = JitterRng::from_clock();
        let mut last_error: Option<GrpcOutboundError> = None;

        for attempt in 0..max_attempts {
            // Trim per-attempt deadline so the inner call honors
            // whatever's left of the caller's overall budget.
            let elapsed = started.elapsed();
            let remaining = match total_budget.checked_sub(elapsed) {
                Some(r) if !r.is_zero() => r,
                _ => {
                    warn!(
                        attempt,
                        elapsed_ms = elapsed.as_millis() as u64,
                        budget_ms  = total_budget.as_millis() as u64,
                        "grpc-retry: deadline exhausted before attempt",
                    );
                    return Err(last_error.unwrap_or_else(|| {
                        GrpcOutboundError::Timeout(
                            "deadline exhausted before retry could be issued".into(),
                        )
                    }));
                }
            };

            let mut req_for_attempt = request.clone();
            req_for_attempt.deadline = remaining;

            let result = self.inner.call_unary(req_for_attempt).await;
            let decision = classify(&result);

            match decision {
                RetryDecision::Success    => return result,
                RetryDecision::Terminal   => {
                    debug!(
                        attempt,
                        outcome = ?result.as_ref().err(),
                        "grpc-retry: terminal failure, surfacing to caller",
                    );
                    return result;
                }
                RetryDecision::Retry | RetryDecision::RetryWithLongerBackoff => {
                    // Capture the error, then decide whether to
                    // sleep + retry or surface it.
                    last_error = result.err();

                    let next_attempt = attempt + 1;
                    if next_attempt >= max_attempts {
                        debug!(
                            attempt = next_attempt,
                            "grpc-retry: max_attempts reached, surfacing last error",
                        );
                        break;
                    }

                    let sleep_for =
                        next_backoff(&self.config, attempt, decision, rng.next_unit());

                    // Don't sleep into the deadline.  If the
                    // backoff alone would push us past the
                    // budget, abandon retries.
                    let elapsed_after = started.elapsed();
                    if elapsed_after.checked_add(sleep_for)
                        .map_or(true, |t| t >= total_budget)
                    {
                        debug!(
                            attempt,
                            sleep_ms     = sleep_for.as_millis() as u64,
                            elapsed_ms   = elapsed_after.as_millis() as u64,
                            budget_ms    = total_budget.as_millis() as u64,
                            "grpc-retry: backoff would exceed deadline, abandoning",
                        );
                        break;
                    }

                    trace!(
                        attempt,
                        sleep_ms = sleep_for.as_millis() as u64,
                        is_quota = decision.is_resource_exhausted(),
                        "grpc-retry: sleeping before next attempt",
                    );
                    tokio::time::sleep(sleep_for).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            GrpcOutboundError::Internal(
                "grpc-retry: exhausted attempts with no recorded error".into(),
            )
        }))
    }
}
