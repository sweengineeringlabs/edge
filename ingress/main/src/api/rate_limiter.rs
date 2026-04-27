//! Rate limiter contract — trait and configuration types.

use crate::api::ingress_error::IngressResult;

/// Contract for a rate limiter.
pub trait RateLimiter: Send + Sync {
    /// Attempt to consume one token; returns an error when the bucket is empty.
    fn try_acquire(&self) -> IngressResult<()>;
    /// Return the number of tokens currently available.
    fn available_tokens(&self) -> u64;
}

/// Builder for a rate limiter specification.
pub struct RateLimiterBuilder {
    capacity: u64,
    refill_rate: f64,
}

impl RateLimiterBuilder {
    /// Create a builder with defaults: capacity 100, refill rate 10/s.
    pub fn new() -> Self { Self { capacity: 100, refill_rate: 10.0 } }

    /// Set the bucket capacity (minimum 1).
    pub fn capacity(mut self, capacity: u64) -> Self { self.capacity = capacity.max(1); self }

    /// Set the token refill rate per second (minimum 0.001).
    pub fn refill_rate(mut self, rate: f64) -> Self { self.refill_rate = rate.max(0.001); self }

    /// Finalise the spec.
    pub fn build(self) -> RateLimiterSpec {
        RateLimiterSpec { capacity: self.capacity, refill_rate: self.refill_rate }
    }
}

impl Default for RateLimiterBuilder {
    fn default() -> Self { Self::new() }
}

/// Finalized rate-limiter configuration.
pub struct RateLimiterSpec {
    pub(crate) capacity: u64,
    pub(crate) refill_rate: f64,
}

impl RateLimiterSpec {
    /// Return the configured bucket capacity.
    pub fn capacity(&self) -> u64 { self.capacity }

    /// Return the configured refill rate (tokens per second).
    pub fn refill_rate(&self) -> f64 { self.refill_rate }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: capacity
    #[test]
    fn test_capacity_returns_configured_capacity() {
        let spec = RateLimiterBuilder::new().capacity(50).build();
        assert_eq!(spec.capacity(), 50);
    }

    /// @covers: refill_rate
    #[test]
    fn test_refill_rate_returns_configured_rate() {
        let spec = RateLimiterBuilder::new().refill_rate(5.0).build();
        assert!((spec.refill_rate() - 5.0).abs() < f64::EPSILON);
    }

    /// @covers: build
    #[test]
    fn test_build_returns_spec_with_default_values() {
        let spec = RateLimiterBuilder::new().build();
        assert_eq!(spec.capacity(), 100);
    }
}
