//! [`GrpcOutbound`] impl for [`GrpcBreakerClient`].
//!
//! - `call_unary`: admit → call inner → record outcome.  When
//!   admission is rejected, return `Unavailable` immediately
//!   without touching the inner client.
//! - `call_stream`: passes through.  Streaming has no clean
//!   single-outcome to record against the breaker; consumers
//!   that want streaming protection should layer their own
//!   per-message logic.
//! - `health_check`: passes through.  A health check is itself
//!   a probe; running it through the breaker would create a
//!   chicken-and-egg dance where the breaker's own state
//!   depends on a call the breaker is gating.

use futures::future::BoxFuture;
use swe_edge_egress_grpc::{
    GrpcMessageStream, GrpcMetadata, GrpcOutbound, GrpcOutboundError, GrpcOutboundResult,
    GrpcRequest, GrpcResponse,
};

use crate::api::breaker_client::GrpcBreakerClient;
use crate::api::breaker_state::Admission;
use crate::api::failure_kind::classify;
use crate::core::transitions::{admit, record};

impl<T: GrpcOutbound + Send + Sync + 'static> GrpcOutbound for GrpcBreakerClient<T> {
    fn call_unary(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcOutboundResult<GrpcResponse>> {
        Box::pin(async move {
            // Admission decision under the lock.
            let decision = {
                let mut node = self.node.lock().await;
                admit(&mut node, &self.config)
            };

            match decision {
                Admission::RejectOpen => {
                    Err(GrpcOutboundError::Unavailable(
                        "grpc-breaker: circuit open, request short-circuited".into(),
                    ))
                }
                Admission::Proceed => {
                    let result = self.inner.call_unary(request).await;
                    let outcome = classify(&result);
                    {
                        let mut node = self.node.lock().await;
                        record(&mut node, &self.config, outcome);
                    }
                    result
                }
            }
        })
    }

    fn call_stream(
        &self,
        method:   String,
        metadata: GrpcMetadata,
        messages: GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcOutboundResult<GrpcMessageStream>> {
        self.inner.call_stream(method, metadata, messages)
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcOutboundResult<()>> {
        self.inner.health_check()
    }
}
