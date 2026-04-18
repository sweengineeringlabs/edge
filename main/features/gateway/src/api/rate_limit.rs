//! Rate-limit contract — trait and configuration types.
//!
//! The runtime implementation lives in `crate::core::rate_limit` as a
//! `pub(crate)` token-bucket struct that impls the [`RateLimiter`]
//! trait defined here. Consumers obtain a `RateLimiter` via
//! `saf::make_rate_limiter(spec)` which returns `impl RateLimiter` —
//! they never name the concrete bucket type.

use crate::api::middleware::RequestMiddleware;
use crate::api::types::GatewayResult;

/// Contract for a rate limiter.
///
/// Extends [`RequestMiddleware`] so a `RateLimiter` can be attached to
/// a middleware pipeline directly. Also exposes the lower-level
/// `try_acquire` / `available_tokens` surface for callers that want
/// to reason about tokens outside the middleware context.
pub trait RateLimiter: RequestMiddleware {
    /// Attempt to acquire a single token.
    ///
    /// Returns `Ok(())` on success or
    /// `Err(GatewayError::RateLimitExceeded)` when the bucket is empty.
    fn try_acquire(&self) -> GatewayResult<()>;

    /// Approximate number of currently-available tokens.
    fn available_tokens(&self) -> u64;
}

/// Builder for a [`RateLimiterSpec`].
///
/// Defaults match the historical `RateLimiter::new(100, 10.0)` constants.
pub struct RateLimiterBuilder {
    capacity: u64,
    refill_rate: f64,
}

impl RateLimiterBuilder {
    /// Create a builder with default values (capacity = 100, refill rate = 10.0/s).
    pub fn new() -> Self {
        Self {
            capacity: 100,
            refill_rate: 10.0,
        }
    }

    /// Set the bucket's burst capacity. Clamped to a minimum of 1.
    pub fn capacity(mut self, capacity: u64) -> Self {
        self.capacity = capacity.max(1);
        self
    }

    /// Set the refill rate in tokens per second. Clamped to a minimum
    /// of 0.001/s to prevent starvation.
    pub fn refill_rate(mut self, rate: f64) -> Self {
        self.refill_rate = rate.max(0.001);
        self
    }

    /// Finalize into a [`RateLimiterSpec`] that `saf::make_rate_limiter`
    /// can consume.
    pub fn build(self) -> RateLimiterSpec {
        RateLimiterSpec {
            capacity: self.capacity,
            refill_rate: self.refill_rate,
        }
    }
}

impl Default for RateLimiterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Finalized rate-limiter configuration.
///
/// Fields are `pub(crate)` so `crate::core::rate_limit` can read them
/// when constructing the concrete bucket. Consumers treat this type
/// as opaque — pass it to `saf::make_rate_limiter`.
pub struct RateLimiterSpec {
    pub(crate) capacity: u64,
    pub(crate) refill_rate: f64,
}

impl RateLimiterSpec {
    /// Return the configured capacity. Exposed for logging / metrics.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Return the configured refill rate.
    pub fn refill_rate(&self) -> f64 {
        self.refill_rate
    }
}
