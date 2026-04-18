//! Token-bucket rate limiter — runtime implementation.
//!
//! Public contract (trait + builder + spec) lives in
//! `crate::api::rate_limit`. This file holds the `pub(crate)` runtime
//! struct that implements that contract.

use async_trait::async_trait;
use parking_lot::Mutex;
use std::time::Instant;

use crate::api::middleware::RequestMiddleware;
use crate::api::rate_limit::{RateLimiter, RateLimiterSpec};
use crate::api::types::{GatewayError, GatewayResult};

/// Internal mutable state of the token bucket.
struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

/// Token-bucket rate limiter.
///
/// `pub(crate)` per rule 50 — consumers reach this type only via the
/// [`RateLimiter`](crate::api::rate_limit::RateLimiter) trait through
/// `saf::make_rate_limiter`.
pub(crate) struct TokenBucketRateLimiter {
    state: Mutex<BucketState>,
    capacity: u64,
    refill_rate: f64,
}

/// Build a [`TokenBucketRateLimiter`] from an api/ spec.
pub(crate) fn build_rate_limiter(spec: RateLimiterSpec) -> TokenBucketRateLimiter {
    TokenBucketRateLimiter {
        state: Mutex::new(BucketState {
            tokens: spec.capacity as f64,
            last_refill: Instant::now(),
        }),
        capacity: spec.capacity,
        refill_rate: spec.refill_rate,
    }
}

impl RateLimiter for TokenBucketRateLimiter {
    fn try_acquire(&self) -> GatewayResult<()> {
        let mut state = self.state.lock();
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();

        let new_tokens = state.tokens + elapsed * self.refill_rate;
        state.tokens = new_tokens.min(self.capacity as f64);
        state.last_refill = now;

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            Ok(())
        } else {
            Err(GatewayError::RateLimitExceeded(format!(
                "capacity {}, refill rate {}/s — try again shortly",
                self.capacity, self.refill_rate
            )))
        }
    }

    fn available_tokens(&self) -> u64 {
        let mut state = self.state.lock();
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        let new_tokens = state.tokens + elapsed * self.refill_rate;
        let clamped = new_tokens.min(self.capacity as f64);
        state.tokens = clamped;
        state.last_refill = now;
        clamped.floor() as u64
    }
}

#[async_trait]
impl RequestMiddleware for TokenBucketRateLimiter {
    async fn process_request(
        &self,
        request: serde_json::Value,
    ) -> GatewayResult<serde_json::Value> {
        self.try_acquire()?;
        Ok(request)
    }
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::rate_limit::RateLimiterBuilder;

    fn build(capacity: u64, rate: f64) -> TokenBucketRateLimiter {
        build_rate_limiter(
            RateLimiterBuilder::new()
                .capacity(capacity)
                .refill_rate(rate)
                .build(),
        )
    }

    #[test]
    fn test_try_acquire_succeeds_when_bucket_has_tokens() {
        let limiter = build(5, 0.001);
        for _ in 0..5 {
            assert!(limiter.try_acquire().is_ok());
        }
        // Bucket exhausted.
        assert!(matches!(
            limiter.try_acquire(),
            Err(GatewayError::RateLimitExceeded(_))
        ));
    }

    #[test]
    fn test_builder_defaults() {
        let spec = RateLimiterBuilder::new().build();
        assert_eq!(spec.capacity(), 100);
        assert_eq!(spec.refill_rate(), 10.0);
    }

    #[test]
    fn test_builder_clamps_capacity_min_1() {
        let spec = RateLimiterBuilder::new().capacity(0).build();
        assert_eq!(spec.capacity(), 1);
    }

    #[test]
    fn test_builder_clamps_refill_rate_min() {
        let spec = RateLimiterBuilder::new().refill_rate(0.0).build();
        assert!(spec.refill_rate() >= 0.001);
    }

    #[tokio::test]
    async fn test_request_middleware_blocks_when_empty() {
        let limiter = build(1, 0.001);
        // First passes.
        assert!(limiter
            .process_request(serde_json::json!({}))
            .await
            .is_ok());
        // Second rejected.
        assert!(matches!(
            limiter
                .process_request(serde_json::json!({}))
                .await
                .unwrap_err(),
            GatewayError::RateLimitExceeded(_)
        ));
    }
}
