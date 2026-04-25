//! Retry configuration and strategy types.

use serde::{Deserialize, Serialize};

/// Backoff strategy for retries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackoffStrategy {
    Constant { delay_ms: u64 },
    Linear { initial_ms: u64, increment_ms: u64 },
    Exponential { initial_ms: u64, multiplier: f64, max_ms: u64 },
}

impl BackoffStrategy {
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        match self {
            BackoffStrategy::Constant { delay_ms } => *delay_ms,
            BackoffStrategy::Linear { initial_ms, increment_ms } => {
                initial_ms + increment_ms * attempt as u64
            }
            BackoffStrategy::Exponential { initial_ms, multiplier, max_ms } => {
                let delay = (*initial_ms as f64 * multiplier.powi(attempt as i32)) as u64;
                delay.min(*max_ms)
            }
        }
    }
}

/// Configuration for retry behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential { initial_ms: 100, multiplier: 2.0, max_ms: 10_000 },
            jitter: true,
        }
    }
}

impl RetryConfig {
    pub fn no_retry() -> Self {
        Self { max_attempts: 1, backoff: BackoffStrategy::Constant { delay_ms: 0 }, jitter: false }
    }

    pub fn with_max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n; self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: delay_for_attempt
    #[test]
    fn test_constant_backoff_returns_same_delay_every_attempt() {
        let s = BackoffStrategy::Constant { delay_ms: 200 };
        assert_eq!(s.delay_for_attempt(0), 200);
        assert_eq!(s.delay_for_attempt(5), 200);
    }

    /// @covers: delay_for_attempt
    #[test]
    fn test_exponential_backoff_grows_up_to_max() {
        let s = BackoffStrategy::Exponential { initial_ms: 100, multiplier: 2.0, max_ms: 1_000 };
        assert!(s.delay_for_attempt(10) <= 1_000);
        assert!(s.delay_for_attempt(1) > s.delay_for_attempt(0));
    }

    /// @covers: no_retry
    #[test]
    fn test_no_retry_creates_single_attempt_config() {
        let cfg = RetryConfig::no_retry();
        assert_eq!(cfg.max_attempts, 1);
    }

    /// @covers: with_max_attempts
    #[test]
    fn test_with_max_attempts_sets_max_attempts() {
        let cfg = RetryConfig::default().with_max_attempts(5);
        assert_eq!(cfg.max_attempts, 5);
    }
}
