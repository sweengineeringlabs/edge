//! Retry configuration types — the public contract for retry middleware.
//!
//! The runtime middleware that consumes these types lives in
//! `crate::core::retry` as `pub(crate)` (rule 50). These api/ types are
//! the consumer-facing construction surface: pick a backoff strategy,
//! a predicate, a max-attempt count, finalize with `build()` to get a
//! [`RetryMiddlewareSpec`], and hand that to `saf::wrap_with_retry` to
//! attach the behavior to a middleware pipeline.
//!
//! Rules enforced by this placement:
//!   - rule 50: `RetryMiddleware` (the runtime impl) stays `pub(crate)` in core/.
//!   - rule 159: saf/ functions can take / return these api/ types in
//!               their public signatures without leaking core types.
//!   - rule 160: consumers never name a core type — only api/ types.

use std::sync::Arc;
use std::time::Duration;

use crate::api::types::GatewayError;

/// Strategy for computing the delay between retry attempts.
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// Constant delay between each attempt.
    Fixed {
        /// Delay between retries.
        delay: Duration,
    },
    /// Exponentially increasing delay: `base * 2^attempt`.
    ///
    /// With jitter enabled, a random fraction of the computed delay is added
    /// to spread out concurrent retries (decorrelated jitter).
    Exponential {
        /// Base delay (attempt 0 waits `base`, attempt 1 waits `base * 2`, etc.).
        base: Duration,
        /// If `true`, add uniform random jitter in `[0, computed_delay)`.
        jitter: bool,
    },
}

impl BackoffStrategy {
    /// Compute the sleep duration for the given zero-based `attempt` index.
    ///
    /// Public on the contract type so tests and advanced consumers can
    /// verify scheduling behavior without reaching into core/.
    pub fn compute_delay(&self, attempt: u32) -> Duration {
        match self {
            BackoffStrategy::Fixed { delay } => *delay,
            BackoffStrategy::Exponential { base, jitter } => {
                let multiplier = 2u64.saturating_pow(attempt);
                let base_ms = base.as_millis() as u64;
                let delay_ms = base_ms.saturating_mul(multiplier);

                if *jitter {
                    let nanos = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .subsec_nanos() as u64;
                    let jitter_ms = if delay_ms > 0 { nanos % delay_ms } else { 0 };
                    Duration::from_millis(delay_ms.saturating_add(jitter_ms))
                } else {
                    Duration::from_millis(delay_ms)
                }
            }
        }
    }
}

/// Predicate that decides whether a `GatewayError` is worth retrying.
///
/// The default predicate delegates to [`GatewayError::is_retryable`].
pub type RetryPredicate = Arc<dyn Fn(&GatewayError) -> bool + Send + Sync>;

/// Returns the default retry predicate backed by `GatewayError::is_retryable`.
pub fn default_retry_predicate() -> RetryPredicate {
    Arc::new(|err: &GatewayError| err.is_retryable())
}

/// Builder for a retry specification.
///
/// Defaults:
/// - `max_attempts`: 3
/// - `backoff`: `Exponential { base: 200ms, jitter: true }`
/// - `predicate`: `GatewayError::is_retryable`
pub struct RetryMiddlewareBuilder {
    max_attempts: u32,
    backoff: BackoffStrategy,
    predicate: RetryPredicate,
}

impl RetryMiddlewareBuilder {
    /// Create a builder with production defaults.
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential {
                base: Duration::from_millis(200),
                jitter: true,
            },
            predicate: default_retry_predicate(),
        }
    }

    /// Set the maximum number of total attempts (including the first).
    ///
    /// # Panics
    ///
    /// Panics if `max_attempts` is 0.
    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        assert!(max_attempts > 0, "max_attempts must be at least 1");
        self.max_attempts = max_attempts;
        self
    }

    /// Use a fixed delay between retries.
    pub fn fixed_backoff(mut self, delay: Duration) -> Self {
        self.backoff = BackoffStrategy::Fixed { delay };
        self
    }

    /// Use exponential backoff with optional jitter.
    pub fn exponential_backoff(mut self, base: Duration, jitter: bool) -> Self {
        self.backoff = BackoffStrategy::Exponential { base, jitter };
        self
    }

    /// Override the retry predicate.
    ///
    /// The default predicate uses [`GatewayError::is_retryable`].
    pub fn retry_predicate(
        mut self,
        predicate: impl Fn(&GatewayError) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.predicate = Arc::new(predicate);
        self
    }

    /// Finalize the builder into a [`RetryMiddlewareSpec`] that can be
    /// passed to `saf::wrap_with_retry` to attach retry behavior to a
    /// middleware pipeline.
    pub fn build(self) -> RetryMiddlewareSpec {
        RetryMiddlewareSpec {
            max_attempts: self.max_attempts,
            backoff: self.backoff,
            predicate: self.predicate,
        }
    }
}

impl Default for RetryMiddlewareBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Finalized retry configuration ready to wrap an inner middleware.
///
/// Produced by [`RetryMiddlewareBuilder::build`]. Opaque-by-design:
/// the fields are accessed only by `saf::wrap_with_retry` (via
/// crate-private accessors in [`crate::core::retry`]). Consumers hold
/// `RetryMiddlewareSpec` values and pass them around — they never need
/// to inspect the internals.
pub struct RetryMiddlewareSpec {
    pub(crate) max_attempts: u32,
    pub(crate) backoff: BackoffStrategy,
    pub(crate) predicate: RetryPredicate,
}

impl RetryMiddlewareSpec {
    /// Return the configured max-attempts count. Exposed for advanced
    /// consumers that need to inspect a spec (e.g. logging, metrics).
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Return a reference to the configured backoff strategy.
    pub fn backoff(&self) -> &BackoffStrategy {
        &self.backoff
    }
}
