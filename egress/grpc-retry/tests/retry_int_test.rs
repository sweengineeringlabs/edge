//! Integration tests for the gRPC retry decorator.
//!
//! These exercise the public API end-to-end: a stub
//! [`GrpcOutbound`] is wrapped with [`GrpcRetryClient`] and the
//! retry loop is observed via call counters and elapsed time.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::BoxFuture;
use swe_edge_egress_grpc::{
    GrpcMetadata, GrpcOutbound, GrpcOutboundError, GrpcOutboundResult, GrpcRequest,
    GrpcResponse, GrpcStatusCode,
};
use swe_edge_egress_grpc_retry::{builder, GrpcRetryClient, GrpcRetryConfig};

/// Stub `GrpcOutbound` that returns a scripted sequence of
/// outcomes and counts how many times `call_unary` was invoked.
///
/// The wrapping `Arc` lets the test observe call counts after
/// the stub has been moved into the decorator.
struct ScriptedClient {
    calls:  AtomicU32,
    /// Each entry is reused on subsequent calls past the script
    /// length; index = (call_count - 1).min(script.len() - 1).
    script: Vec<Outcome>,
}

#[derive(Clone, Debug)]
enum Outcome {
    Ok,
    Status(GrpcStatusCode, &'static str),
    Unavailable(&'static str),
    Internal(&'static str),
}

impl ScriptedClient {
    fn new(script: Vec<Outcome>) -> Self {
        Self { calls: AtomicU32::new(0), script }
    }

    fn call_count(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }

    async fn dispatch(&self) -> GrpcOutboundResult<GrpcResponse> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        let idx = (n as usize).min(self.script.len().saturating_sub(1));
        let outcome = self.script.get(idx).cloned().unwrap_or(Outcome::Ok);
        match outcome {
            Outcome::Ok => Ok(GrpcResponse {
                body:     b"ok".to_vec(),
                metadata: GrpcMetadata::default(),
            }),
            Outcome::Status(code, msg) => Err(GrpcOutboundError::Status(code, msg.into())),
            Outcome::Unavailable(msg) => Err(GrpcOutboundError::Unavailable(msg.into())),
            Outcome::Internal(msg) => Err(GrpcOutboundError::Internal(msg.into())),
        }
    }
}

/// Newtype that owns an `Arc<ScriptedClient>` and implements
/// [`GrpcOutbound`].  Avoids needing a blanket impl on `Arc<T>`,
/// which would conflict with the orphan rule.
struct SharedClient<T> {
    inner: Arc<T>,
}

impl<T> SharedClient<T> {
    fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }
}

impl GrpcOutbound for SharedClient<ScriptedClient> {
    fn call_unary(
        &self,
        _request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcOutboundResult<GrpcResponse>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move { inner.dispatch().await })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcOutboundResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

fn fast_config() -> GrpcRetryConfig {
    GrpcRetryConfig::from_config(
        r#"
            max_attempts = 5
            initial_backoff_ms = 1
            backoff_multiplier = 1.0
            jitter_factor = 0.0
            max_backoff_ms = 1
        "#,
    )
    .expect("test config")
}

fn make_request(deadline: Duration) -> GrpcRequest {
    GrpcRequest::new("svc.Test/Method", b"hello".to_vec(), deadline)
}

fn wrap(
    inner: Arc<ScriptedClient>,
    config: GrpcRetryConfig,
) -> GrpcRetryClient<SharedClient<ScriptedClient>> {
    GrpcRetryClient::new(SharedClient::new(inner), config)
}

/// @covers: builder — SWE default loads.
#[tokio::test(flavor = "multi_thread")]
async fn test_builder_loads_swe_default() {
    let b = builder().expect("baseline parses");
    assert!(b.config().max_attempts >= 1);
}

/// @covers: GrpcRetryClient — first-call success short-circuits.
#[tokio::test(flavor = "multi_thread")]
async fn test_first_attempt_success_does_not_retry() {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Ok]));
    let client = wrap(inner.clone(), fast_config());
    let resp = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .expect("ok");
    assert_eq!(resp.body, b"ok");
    assert_eq!(inner.call_count(), 1, "no retry on success");
}

/// @covers: classify(PermissionDenied) — NEVER retried.
///
/// **Acceptance test gate**: hand a stub returning
/// `PermissionDenied`, assert call_count == 1.
#[tokio::test(flavor = "multi_thread")]
async fn test_permission_denied_is_never_retried() {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Status(
        GrpcStatusCode::PermissionDenied,
        "no",
    )]));
    let client = wrap(inner.clone(), fast_config());
    let err = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .expect_err("must fail");
    match err {
        GrpcOutboundError::Status(GrpcStatusCode::PermissionDenied, _) => {}
        other => panic!("expected PermissionDenied, got {other:?}"),
    }
    assert_eq!(
        inner.call_count(),
        1,
        "PermissionDenied must not be retried (called {} times)",
        inner.call_count(),
    );
}

/// @covers: classify(Unauthenticated) — NEVER retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_unauthenticated_is_never_retried() {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Status(
        GrpcStatusCode::Unauthenticated,
        "bad token",
    )]));
    let client = wrap(inner.clone(), fast_config());
    let err = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .expect_err("must fail");
    assert!(matches!(
        err,
        GrpcOutboundError::Status(GrpcStatusCode::Unauthenticated, _)
    ));
    assert_eq!(inner.call_count(), 1, "Unauthenticated must not be retried");
}

/// @covers: classify(Unavailable) — retries until success.
#[tokio::test(flavor = "multi_thread")]
async fn test_unavailable_then_ok_succeeds_after_retry() {
    let inner = Arc::new(ScriptedClient::new(vec![
        Outcome::Status(GrpcStatusCode::Unavailable, "blip"),
        Outcome::Status(GrpcStatusCode::Unavailable, "blip"),
        Outcome::Ok,
    ]));
    let client = wrap(inner.clone(), fast_config());
    let resp = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .expect("eventually ok");
    assert_eq!(resp.body, b"ok");
    assert_eq!(inner.call_count(), 3, "two retries before the Ok");
}

/// @covers: classify(Unavailable) — exhausts attempts.
#[tokio::test(flavor = "multi_thread")]
async fn test_unavailable_throughout_exhausts_max_attempts() {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Status(
        GrpcStatusCode::Unavailable,
        "down",
    )]));
    let client = wrap(inner.clone(), fast_config());
    let err = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .expect_err("never recovers");
    assert!(matches!(
        err,
        GrpcOutboundError::Status(GrpcStatusCode::Unavailable, _)
    ));
    assert_eq!(inner.call_count(), 5, "exhausts max_attempts");
}

/// @covers: classify(Internal) — terminal, no retry.
#[tokio::test(flavor = "multi_thread")]
async fn test_internal_is_terminal() {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Status(
        GrpcStatusCode::Internal,
        "bug",
    )]));
    let client = wrap(inner.clone(), fast_config());
    let _ = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .expect_err("terminal");
    assert_eq!(inner.call_count(), 1, "Internal must not be retried");
}

/// @covers: classify(DeadlineExceeded) — terminal.
#[tokio::test(flavor = "multi_thread")]
async fn test_deadline_exceeded_is_terminal() {
    let inner = Arc::new(ScriptedClient::new(vec![Outcome::Status(
        GrpcStatusCode::DeadlineExceeded,
        "tick",
    )]));
    let client = wrap(inner.clone(), fast_config());
    let _ = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .expect_err("terminal");
    assert_eq!(
        inner.call_count(),
        1,
        "DeadlineExceeded counts retry budget — not retried",
    );
}

/// @covers: classify(transport Unavailable) → retried.
#[tokio::test(flavor = "multi_thread")]
async fn test_transport_unavailable_retries_then_internal_terminates() {
    let inner = Arc::new(ScriptedClient::new(vec![
        Outcome::Unavailable("transport blip"),
        Outcome::Internal("not retryable"),
    ]));
    let client = wrap(inner.clone(), fast_config());
    let _ = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .expect_err("Internal is terminal");
    assert_eq!(inner.call_count(), 2);
}

/// @covers: classify(ResourceExhausted) — retried with longer backoff.
#[tokio::test(flavor = "multi_thread")]
async fn test_resource_exhausted_retries_with_longer_backoff() {
    let inner = Arc::new(ScriptedClient::new(vec![
        Outcome::Status(GrpcStatusCode::ResourceExhausted, "quota"),
        Outcome::Status(GrpcStatusCode::ResourceExhausted, "quota"),
        Outcome::Ok,
    ]));
    let cfg = GrpcRetryConfig::from_config(
        r#"
            max_attempts = 5
            initial_backoff_ms = 5
            backoff_multiplier = 1.0
            jitter_factor = 0.0
            max_backoff_ms = 100
        "#,
    )
    .unwrap();
    let client = GrpcRetryClient::new(SharedClient::new(inner.clone()), cfg);
    let resp = client
        .call_unary(make_request(Duration::from_secs(5)))
        .await
        .expect("eventually ok");
    assert_eq!(resp.body, b"ok");
    assert_eq!(inner.call_count(), 3);
}

/// @covers: deadline budget — retry stops when budget exhausted.
///
/// **Acceptance test gate**: total attempts ≤ deadline / initial_backoff.
///
/// Slow stub: each call sleeps for ~50ms before failing with
/// `Unavailable`.  The total deadline is 200ms.  Even though the
/// config allows up to 100 attempts, the retry loop must abandon
/// when the deadline is exhausted.
#[tokio::test(flavor = "multi_thread")]
async fn test_retry_honors_caller_deadline_as_total_budget() {
    struct Slow {
        calls: AtomicU32,
        per_call: Duration,
    }

    struct SharedSlow(Arc<Slow>);

    impl GrpcOutbound for SharedSlow {
        fn call_unary(
            &self,
            _r: GrpcRequest,
        ) -> BoxFuture<'_, GrpcOutboundResult<GrpcResponse>> {
            let inner = Arc::clone(&self.0);
            Box::pin(async move {
                inner.calls.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(inner.per_call).await;
                Err(GrpcOutboundError::Status(
                    GrpcStatusCode::Unavailable,
                    "slow blip".into(),
                ))
            })
        }
        fn health_check(&self) -> BoxFuture<'_, GrpcOutboundResult<()>> {
            Box::pin(async { Ok(()) })
        }
    }

    let stats = Arc::new(Slow {
        calls:    AtomicU32::new(0),
        per_call: Duration::from_millis(50),
    });

    let cfg = GrpcRetryConfig::from_config(
        r#"
            max_attempts = 100
            initial_backoff_ms = 10
            backoff_multiplier = 1.0
            jitter_factor = 0.0
            max_backoff_ms = 10
        "#,
    )
    .unwrap();
    let client = GrpcRetryClient::new(SharedSlow(stats.clone()), cfg);

    let total_budget = Duration::from_millis(200);
    let started = Instant::now();
    let _ = client
        .call_unary(make_request(total_budget))
        .await
        .expect_err("budget exhausts");
    let elapsed = started.elapsed();

    let calls = stats.calls.load(Ordering::SeqCst);
    assert!(
        calls < 100,
        "calls ({calls}) reached max_attempts — deadline appears ignored",
    );
    let ceiling = 1 + (total_budget.as_millis() / 10) as u32;
    assert!(
        calls <= ceiling,
        "calls ({calls}) exceeded ceiling deadline / initial_backoff = {ceiling}",
    );
    assert!(
        elapsed < total_budget * 3,
        "retry loop ran for {elapsed:?}, far past budget {total_budget:?}",
    );
}
