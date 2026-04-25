//! Runtime retry middleware — the implementation that backs the
//! [`crate::api::retry`] contract types.
//!
//! All public-surface configuration types (`BackoffStrategy`,
//! `RetryPredicate`, `RetryMiddlewareBuilder`, `RetryMiddlewareSpec`)
//! live in `crate::api::retry`. Consumers never name the runtime
//! `RetryMiddleware` directly — they obtain one indirectly via
//! `saf::wrap_with_retry(spec, inner)`, which returns `impl
//! RequestMiddleware`.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::api::middleware::RequestMiddleware;
use crate::api::retry::RetryMiddlewareSpec;
use crate::api::types::GatewayError;

// ── Sleeper (abstracted for testing) ────────────────────────────────────────

/// Abstraction over `tokio::time::sleep` so unit tests can substitute a
/// zero-cost or observable implementation.
#[async_trait]
pub(crate) trait Sleeper: Send + Sync {
    async fn sleep(&self, duration: Duration);
}

/// Production sleeper that delegates to `tokio::time::sleep`.
pub(crate) struct TokioSleeper;

#[async_trait]
impl Sleeper for TokioSleeper {
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

// ── RetryMiddleware ─────────────────────────────────────────────────────────

/// Runtime retry middleware.
///
/// `pub(crate)` per rule 50: core items are implementation, not public
/// contract. Consumers reach this type only through
/// `saf::wrap_with_retry`, which returns it as `impl RequestMiddleware`.
pub(crate) struct RetryMiddleware {
    max_attempts: u32,
    backoff: crate::api::retry::BackoffStrategy,
    predicate: crate::api::retry::RetryPredicate,
    inner: Arc<dyn RequestMiddleware>,
    sleeper: Arc<dyn Sleeper>,
}

/// Build a [`RetryMiddleware`] from an api/ spec and an inner middleware.
///
/// This is the bridge between the public builder surface in `api/` and
/// the runtime implementation here. Called from `saf::wrap_with_retry`.
pub(crate) fn build_retry_middleware(
    spec: RetryMiddlewareSpec,
    inner: Arc<dyn RequestMiddleware>,
) -> RetryMiddleware {
    build_retry_middleware_with_sleeper(spec, inner, Arc::new(TokioSleeper))
}

/// Same as [`build_retry_middleware`] but accepts a custom sleeper —
/// used by tests to avoid real time.sleep calls.
pub(crate) fn build_retry_middleware_with_sleeper(
    spec: RetryMiddlewareSpec,
    inner: Arc<dyn RequestMiddleware>,
    sleeper: Arc<dyn Sleeper>,
) -> RetryMiddleware {
    RetryMiddleware {
        max_attempts: spec.max_attempts,
        backoff: spec.backoff,
        predicate: spec.predicate,
        inner,
        sleeper,
    }
}

#[async_trait]
impl RequestMiddleware for RetryMiddleware {
    async fn process_request(
        &self,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, GatewayError> {
        let mut last_error: Option<GatewayError> = None;

        for attempt in 0..self.max_attempts {
            let req_clone = request.clone();

            match self.inner.process_request(req_clone).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    let is_last_attempt = attempt + 1 >= self.max_attempts;
                    let is_retryable = (self.predicate)(&err);

                    if is_last_attempt || !is_retryable {
                        return Err(err);
                    }

                    tracing::warn!(
                        attempt = attempt + 1,
                        max_attempts = self.max_attempts,
                        error = %err,
                        "retrying transient gateway error"
                    );

                    let delay = self.backoff.compute_delay(attempt);
                    self.sleeper.sleep(delay).await;

                    last_error = Some(err);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            GatewayError::internal("retry loop exited without result or error")
        }))
    }
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::retry::{BackoffStrategy, RetryMiddlewareBuilder};
    use parking_lot::Mutex;

    struct RecordingSleeper {
        recorded: Mutex<Vec<Duration>>,
    }

    impl RecordingSleeper {
        fn new() -> Self {
            Self { recorded: Mutex::new(Vec::new()) }
        }

        fn recorded_delays(&self) -> Vec<Duration> {
            self.recorded.lock().clone()
        }
    }

    #[async_trait]
    impl Sleeper for RecordingSleeper {
        async fn sleep(&self, duration: Duration) {
            self.recorded.lock().push(duration);
        }
    }

    struct FailNThenSucceed {
        remaining_failures: Mutex<u32>,
        error_factory: Box<dyn Fn() -> GatewayError + Send + Sync>,
        success_value: serde_json::Value,
    }

    impl FailNThenSucceed {
        fn new(
            failures: u32,
            error_factory: impl Fn() -> GatewayError + Send + Sync + 'static,
            success_value: serde_json::Value,
        ) -> Self {
            Self {
                remaining_failures: Mutex::new(failures),
                error_factory: Box::new(error_factory),
                success_value,
            }
        }
    }

    #[async_trait]
    impl RequestMiddleware for FailNThenSucceed {
        async fn process_request(
            &self,
            _request: serde_json::Value,
        ) -> Result<serde_json::Value, GatewayError> {
            let mut remaining = self.remaining_failures.lock();
            if *remaining > 0 {
                *remaining -= 1;
                Err((self.error_factory)())
            } else {
                Ok(self.success_value.clone())
            }
        }
    }

    struct AlwaysFail {
        error_factory: Box<dyn Fn() -> GatewayError + Send + Sync>,
    }

    impl AlwaysFail {
        fn new(error_factory: impl Fn() -> GatewayError + Send + Sync + 'static) -> Self {
            Self { error_factory: Box::new(error_factory) }
        }
    }

    #[async_trait]
    impl RequestMiddleware for AlwaysFail {
        async fn process_request(
            &self,
            _request: serde_json::Value,
        ) -> Result<serde_json::Value, GatewayError> {
            Err((self.error_factory)())
        }
    }

    fn build_with_recording(
        builder: RetryMiddlewareBuilder,
        inner: Arc<dyn RequestMiddleware>,
        sleeper: Arc<RecordingSleeper>,
    ) -> RetryMiddleware {
        build_retry_middleware_with_sleeper(builder.build(), inner, sleeper)
    }

    #[test]
    fn test_compute_delay_fixed_returns_constant() {
        let strategy = BackoffStrategy::Fixed { delay: Duration::from_millis(100) };
        assert_eq!(strategy.compute_delay(0), Duration::from_millis(100));
        assert_eq!(strategy.compute_delay(1), Duration::from_millis(100));
        assert_eq!(strategy.compute_delay(5), Duration::from_millis(100));
    }

    #[test]
    fn test_compute_delay_exponential_without_jitter_doubles() {
        let strategy = BackoffStrategy::Exponential { base: Duration::from_millis(100), jitter: false };
        assert_eq!(strategy.compute_delay(0), Duration::from_millis(100));
        assert_eq!(strategy.compute_delay(1), Duration::from_millis(200));
        assert_eq!(strategy.compute_delay(2), Duration::from_millis(400));
        assert_eq!(strategy.compute_delay(3), Duration::from_millis(800));
    }

    #[test]
    fn test_compute_delay_exponential_with_jitter_at_least_base() {
        let strategy = BackoffStrategy::Exponential { base: Duration::from_millis(100), jitter: true };
        for attempt in 0..5 {
            let delay = strategy.compute_delay(attempt);
            let min = Duration::from_millis(100 * 2u64.pow(attempt));
            assert!(delay >= min, "attempt {attempt}: delay {delay:?} < min {min:?}");
        }
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_transient_failures() {
        let sleeper = Arc::new(RecordingSleeper::new());
        let inner: Arc<dyn RequestMiddleware> = Arc::new(FailNThenSucceed::new(
            2,
            || GatewayError::unavailable("service down"),
            serde_json::json!({"ok": true}),
        ));

        let mw = build_with_recording(
            RetryMiddlewareBuilder::new()
                .max_attempts(3)
                .fixed_backoff(Duration::from_millis(50)),
            inner,
            sleeper.clone(),
        );

        let result = mw.process_request(serde_json::json!({})).await;
        assert!(result.is_ok(), "should succeed on 3rd attempt");
        assert_eq!(result.unwrap(), serde_json::json!({"ok": true}));

        let delays = sleeper.recorded_delays();
        assert_eq!(delays.len(), 2);
        assert_eq!(delays[0], Duration::from_millis(50));
        assert_eq!(delays[1], Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_retry_gives_up_after_max_attempts() {
        let sleeper = Arc::new(RecordingSleeper::new());
        let inner: Arc<dyn RequestMiddleware> =
            Arc::new(AlwaysFail::new(|| GatewayError::timeout("request timed out")));

        let mw = build_with_recording(
            RetryMiddlewareBuilder::new()
                .max_attempts(3)
                .fixed_backoff(Duration::from_millis(10)),
            inner,
            sleeper.clone(),
        );

        let result = mw.process_request(serde_json::json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, GatewayError::Timeout(_)), "got {err:?}");

        assert_eq!(sleeper.recorded_delays().len(), 2);
    }

    #[tokio::test]
    async fn test_retry_does_not_retry_non_retryable_error() {
        let sleeper = Arc::new(RecordingSleeper::new());
        let inner: Arc<dyn RequestMiddleware> =
            Arc::new(AlwaysFail::new(|| GatewayError::not_found("resource missing")));

        let mw = build_with_recording(
            RetryMiddlewareBuilder::new()
                .max_attempts(5)
                .fixed_backoff(Duration::from_millis(10)),
            inner,
            sleeper.clone(),
        );

        let result = mw.process_request(serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GatewayError::NotFound(_)));
        assert!(sleeper.recorded_delays().is_empty());
    }

    #[tokio::test]
    async fn test_retry_exponential_backoff_delays_increase() {
        let sleeper = Arc::new(RecordingSleeper::new());
        let inner: Arc<dyn RequestMiddleware> = Arc::new(AlwaysFail::new(|| {
            GatewayError::ConnectionFailed("connection refused".into())
        }));

        let mw = build_with_recording(
            RetryMiddlewareBuilder::new()
                .max_attempts(4)
                .exponential_backoff(Duration::from_millis(100), false),
            inner,
            sleeper.clone(),
        );

        let _ = mw.process_request(serde_json::json!({})).await;

        let delays = sleeper.recorded_delays();
        assert_eq!(delays.len(), 3);
        assert_eq!(delays[0], Duration::from_millis(100));
        assert_eq!(delays[1], Duration::from_millis(200));
        assert_eq!(delays[2], Duration::from_millis(400));
    }

    #[tokio::test]
    async fn test_retry_custom_predicate_overrides_default() {
        let sleeper = Arc::new(RecordingSleeper::new());
        let inner: Arc<dyn RequestMiddleware> = Arc::new(FailNThenSucceed::new(
            1,
            || GatewayError::not_found("temporary 404"),
            serde_json::json!({"found": true}),
        ));

        let mw = build_with_recording(
            RetryMiddlewareBuilder::new()
                .max_attempts(3)
                .fixed_backoff(Duration::from_millis(10))
                .retry_predicate(|err| matches!(err, GatewayError::NotFound(_))),
            inner,
            sleeper.clone(),
        );

        let result = mw.process_request(serde_json::json!({})).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!({"found": true}));
        assert_eq!(sleeper.recorded_delays().len(), 1);
    }

    #[tokio::test]
    async fn test_retry_max_attempts_one_does_not_retry() {
        let sleeper = Arc::new(RecordingSleeper::new());
        let inner: Arc<dyn RequestMiddleware> =
            Arc::new(AlwaysFail::new(|| GatewayError::unavailable("down")));

        let mw = build_with_recording(
            RetryMiddlewareBuilder::new().max_attempts(1),
            inner,
            sleeper.clone(),
        );

        let result = mw.process_request(serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(sleeper.recorded_delays().is_empty());
    }

    #[test]
    #[should_panic(expected = "max_attempts must be at least 1")]
    fn test_builder_panics_on_zero_max_attempts() {
        RetryMiddlewareBuilder::new().max_attempts(0);
    }

    #[test]
    fn test_builder_default_matches_new() {
        let b = RetryMiddlewareBuilder::default();
        let spec = b.build();
        assert_eq!(spec.max_attempts(), 3);
        assert!(matches!(spec.backoff(), BackoffStrategy::Exponential { jitter: true, .. }));
    }
}
