//! Impl blocks for [`RetryLayer`] — constructor + the
//! [`reqwest_middleware::Middleware`] trait impl.
//!
//! The middleware loop honors the full [`RetryConfig`]: method
//! filtering, status filtering, exponential backoff with
//! capped max interval, max-retries budget. Composes with the
//! rest of the middleware chain — `next.run()` is called on
//! every attempt, so downstream layers (cache/auth/etc.) see
//! each retry as a fresh dispatch.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::api::retry_config::RetryConfig;
use crate::api::retry_layer::RetryLayer;

impl RetryLayer {
    /// Construct from a resolved config.
    pub(crate) fn new(config: RetryConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Compute the backoff delay for retry attempt `n`
    /// (0-indexed: `n=0` is the wait BEFORE the first retry,
    /// i.e. after the original attempt failed). Capped at
    /// `max_interval_ms`.
    fn backoff_for(&self, attempt: u32) -> Duration {
        let initial_ms = self.config.initial_interval_ms as f64;
        let multiplier = self.config.multiplier;
        let max_ms = self.config.max_interval_ms;
        let ms = (initial_ms * multiplier.powi(attempt as i32)).round() as u64;
        Duration::from_millis(ms.min(max_ms))
    }

    /// Is this method eligible for retry per config?
    fn method_retryable(&self, method: &reqwest::Method) -> bool {
        let method_str = method.as_str();
        self.config
            .retryable_methods
            .iter()
            .any(|m| m.eq_ignore_ascii_case(method_str))
    }

    /// Is this status eligible for retry per config?
    fn status_retryable(&self, status: reqwest::StatusCode) -> bool {
        self.config.retryable_statuses.contains(&status.as_u16())
    }

    /// Should we retry given the outcome of an attempt?
    /// Separated out so the middleware's control flow stays
    /// readable + this logic is unit-testable without spinning
    /// up reqwest.
    fn should_retry(
        &self,
        outcome: &Result<reqwest::StatusCode, bool>,
    ) -> bool {
        match outcome {
            // Success-at-HTTP-layer: retry iff status is in the
            // configured retryable set.
            Ok(status) => self.status_retryable(*status),
            // Transport error: `true` = transient, retry; `false`
            // = not transient (DNS hard-fail, TLS cert, etc.),
            // don't retry.
            Err(is_transient) => *is_transient,
        }
    }
}

#[async_trait]
impl reqwest_middleware::Middleware for RetryLayer {
    async fn handle(
        &self,
        req: reqwest::Request,
        ext: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        // If the method isn't retryable, pass through — avoids
        // cloning a request we'll never re-send.
        if !self.method_retryable(req.method()) {
            return next.run(req, ext).await;
        }

        // total attempts = 1 original + N retries.
        let total = self.config.max_retries.saturating_add(1);

        // Try to clone the request up front. If the body isn't
        // cloneable (streaming), fall back to one-shot — the
        // retry promise doesn't apply.
        let cloneable = req.try_clone().is_some();
        if !cloneable {
            return next.run(req, ext).await;
        }

        let mut last_result: Option<reqwest_middleware::Result<reqwest::Response>> = None;
        for attempt in 0..total {
            if attempt > 0 {
                let delay = self.backoff_for(attempt - 1);
                tokio::time::sleep(delay).await;
            }

            // try_clone succeeds because we pre-checked above.
            let attempt_req = req.try_clone().expect("body is cloneable — checked earlier");
            let attempt_next = next.clone();
            let result = attempt_next.run(attempt_req, ext).await;

            let retry = match &result {
                Ok(resp) => self.status_retryable(resp.status()),
                Err(e) => is_transient_error(e),
            };

            // Last attempt — return whatever we got.
            if attempt + 1 == total {
                return result;
            }

            if !retry {
                // Don't retry — return immediately.
                return result;
            }

            last_result = Some(result);
        }

        // Unreachable: the `attempt + 1 == total` branch above
        // returns on the final iteration. Keep a sensible value
        // to satisfy the type checker.
        last_result.expect("loop always populates on the final iteration")
    }
}

/// Classify a `reqwest_middleware::Error` as transient
/// (retry-worthy) or non-transient. Transient failures:
/// connection reset, DNS temporary failures, read timeout, etc.
/// Non-transient: certificate errors, malformed URL.
fn is_transient_error(err: &reqwest_middleware::Error) -> bool {
    match err {
        reqwest_middleware::Error::Reqwest(inner) => {
            // Most reqwest errors stem from I/O and are worth
            // retrying. Certificate / redirect / builder errors
            // are not. `is_connect` catches connect-time issues
            // (good to retry); `is_timeout` is transient by
            // definition; everything else we treat as transient
            // unless clearly a config issue.
            inner.is_timeout() || inner.is_connect() || inner.is_request()
        }
        // Middleware-level errors (e.g. from our auth layer)
        // are generally NOT transient — they reflect
        // configuration or credential problems. Don't retry.
        reqwest_middleware::Error::Middleware(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RetryConfig {
        RetryConfig::from_config(
            r#"
                max_retries = 3
                initial_interval_ms = 200
                max_interval_ms = 10000
                multiplier = 2.0
                retryable_statuses = [429, 500, 502, 503, 504]
                retryable_methods = ["GET", "HEAD", "PUT", "DELETE"]
            "#,
        )
        .unwrap()
    }

    /// @covers: RetryLayer::backoff_for
    #[test]
    fn test_backoff_grows_exponentially() {
        let l = RetryLayer::new(test_config());
        assert_eq!(l.backoff_for(0), Duration::from_millis(200));
        assert_eq!(l.backoff_for(1), Duration::from_millis(400));
        assert_eq!(l.backoff_for(2), Duration::from_millis(800));
    }

    /// @covers: RetryLayer::backoff_for
    #[test]
    fn test_backoff_caps_at_max_interval() {
        let l = RetryLayer::new(test_config());
        assert_eq!(l.backoff_for(10), Duration::from_millis(10000));
    }

    /// @covers: RetryLayer::method_retryable
    #[test]
    fn test_method_retryable_matches_config() {
        let l = RetryLayer::new(test_config());
        assert!(l.method_retryable(&reqwest::Method::GET));
        assert!(l.method_retryable(&reqwest::Method::DELETE));
        assert!(!l.method_retryable(&reqwest::Method::POST));
        assert!(!l.method_retryable(&reqwest::Method::PATCH));
    }

    /// @covers: RetryLayer::status_retryable
    #[test]
    fn test_status_retryable_matches_config() {
        let l = RetryLayer::new(test_config());
        assert!(l.status_retryable(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(l.status_retryable(reqwest::StatusCode::BAD_GATEWAY));
        assert!(!l.status_retryable(reqwest::StatusCode::OK));
        assert!(!l.status_retryable(reqwest::StatusCode::BAD_REQUEST));
    }

    /// @covers: RetryLayer::should_retry
    #[test]
    fn test_should_retry_on_retryable_status() {
        let l = RetryLayer::new(test_config());
        assert!(l.should_retry(&Ok(reqwest::StatusCode::SERVICE_UNAVAILABLE)));
    }

    /// @covers: RetryLayer::should_retry
    #[test]
    fn test_should_not_retry_on_success_status() {
        let l = RetryLayer::new(test_config());
        assert!(!l.should_retry(&Ok(reqwest::StatusCode::OK)));
    }

    /// @covers: RetryLayer::should_retry
    #[test]
    fn test_should_not_retry_on_client_error_status() {
        let l = RetryLayer::new(test_config());
        assert!(!l.should_retry(&Ok(reqwest::StatusCode::BAD_REQUEST)));
        assert!(!l.should_retry(&Ok(reqwest::StatusCode::UNAUTHORIZED)));
    }

    /// @covers: RetryLayer::should_retry
    #[test]
    fn test_should_retry_transport_error_when_transient() {
        let l = RetryLayer::new(test_config());
        assert!(l.should_retry(&Err(true)));
    }

    /// @covers: RetryLayer::should_retry
    #[test]
    fn test_should_not_retry_transport_error_when_not_transient() {
        let l = RetryLayer::new(test_config());
        assert!(!l.should_retry(&Err(false)));
    }

    // Note: we don't unit-test `is_transient_error` directly
    // because constructing real `reqwest::Error` and
    // `reqwest_middleware::Error::Middleware(anyhow::Error)`
    // values without the anyhow dep is awkward in a test.
    // Behavior is covered by integration tests that actually
    // dispatch against failing hosts.
}
