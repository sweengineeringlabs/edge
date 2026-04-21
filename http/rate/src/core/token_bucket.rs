//! Per-host token bucket.
//!
//! Tokens refill at `tokens_per_second` up to `burst_capacity`.
//! Each request tries to consume one token; if none available,
//! the caller waits until a token is ready (calculated from the
//! refill rate).

use std::time::{Duration, Instant};

use crate::api::rate_config::RateConfig;

/// Token bucket state. Not thread-safe on its own — wrap in a
/// mutex for concurrent use (the middleware does this via moka
/// + tokio::sync::Mutex).
#[derive(Debug)]
pub(crate) struct TokenBucket {
    /// Current token count. Fractional — tokens accumulate
    /// linearly even when refill rate isn't a whole number.
    tokens: f64,
    /// When we last refilled the bucket. Used to compute how
    /// many tokens have accumulated since.
    last_refill: Instant,
}

impl TokenBucket {
    /// Construct a full bucket (consumers shouldn't be
    /// artificially throttled on startup).
    pub(crate) fn full(config: &RateConfig) -> Self {
        Self {
            tokens: config.burst_capacity as f64,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time since last refill.
    /// Caps at `burst_capacity`.
    fn refill(&mut self, config: &RateConfig) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let added = elapsed.as_secs_f64() * config.tokens_per_second as f64;
        self.tokens = (self.tokens + added).min(config.burst_capacity as f64);
        self.last_refill = now;
    }

    /// Try to acquire one token without waiting.
    ///
    /// Returns `Ok(())` if a token was available + consumed.
    /// Returns `Err(wait)` if the bucket is empty; `wait` is
    /// the time until one token will be available.
    pub(crate) fn try_acquire(
        &mut self,
        config: &RateConfig,
    ) -> Result<(), Duration> {
        self.refill(config);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            let deficit = 1.0 - self.tokens;
            let secs_until_one = deficit / config.tokens_per_second as f64;
            Err(Duration::from_secs_f64(secs_until_one))
        }
    }

    #[cfg(test)]
    pub(crate) fn tokens(&self) -> f64 {
        self.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RateConfig {
        RateConfig::from_config(
            r#"
                tokens_per_second = 10
                burst_capacity = 20
                per_host = true
            "#,
        )
        .unwrap()
    }

    /// @covers: TokenBucket::full
    #[test]
    fn test_full_starts_at_burst_capacity() {
        let cfg = test_config();
        let b = TokenBucket::full(&cfg);
        assert_eq!(b.tokens(), 20.0);
    }

    /// @covers: TokenBucket::try_acquire
    #[test]
    fn test_acquire_succeeds_when_tokens_available() {
        let cfg = test_config();
        let mut b = TokenBucket::full(&cfg);
        assert!(b.try_acquire(&cfg).is_ok());
        assert!(b.tokens() < 20.0);
    }

    /// @covers: TokenBucket::try_acquire
    #[test]
    fn test_acquire_exhausts_bucket_and_returns_wait() {
        let cfg = test_config();
        let mut b = TokenBucket::full(&cfg);
        // Drain the bucket (20 tokens).
        for _ in 0..20 {
            assert!(b.try_acquire(&cfg).is_ok());
        }
        // 21st should need to wait — 1 / 10 per second = 100ms.
        match b.try_acquire(&cfg) {
            Err(d) => assert!(d >= Duration::from_millis(90)),
            Ok(_) => panic!("expected wait when bucket exhausted"),
        }
    }

    /// @covers: TokenBucket::refill
    #[test]
    fn test_refill_caps_at_burst_capacity() {
        let cfg = test_config();
        let mut b = TokenBucket::full(&cfg);
        // Simulate time passing by backdating last_refill.
        b.last_refill = Instant::now() - Duration::from_secs(100);
        // Next acquire should refill but cap.
        b.try_acquire(&cfg).unwrap();
        // We consumed 1 from a capped full bucket; should be
        // 19 (20 cap - 1 consumed).
        assert!((b.tokens() - 19.0).abs() < 0.001);
    }

    /// @covers: TokenBucket::try_acquire
    #[test]
    fn test_refill_restores_tokens_proportional_to_elapsed_time() {
        let cfg = test_config();
        let mut b = TokenBucket::full(&cfg);
        // Drain fully.
        for _ in 0..20 {
            b.try_acquire(&cfg).unwrap();
        }
        // Back-date to give "0.5 seconds of refill" = 5 tokens
        // (rate = 10/s).
        b.last_refill = Instant::now() - Duration::from_millis(500);
        b.try_acquire(&cfg).unwrap();
        // Had 5 from refill, consumed 1, so ~4 left.
        assert!(
            (b.tokens() - 4.0).abs() < 0.1,
            "expected ~4 tokens after partial refill, got {}",
            b.tokens()
        );
    }
}
