//! Default impl of [`HttpRetry`](crate::api::http_retry::HttpRetry).
//!
//! Scaffold phase: holds a resolved [`RetryConfig`](crate::api::retry_config::RetryConfig)
//! and answers `describe()`. Real middleware behavior lands
//! when the crate's `Middleware` impl is written — at that
//! point the strategy/policy state moves in here too.

use crate::api::retry_config::RetryConfig;
use crate::api::http_retry::HttpRetry;

/// Default HttpRetry implementation. `pub(crate)` — consumers
/// never touch this type directly; they go through `saf::builder`.
#[derive(Debug)]
pub(crate) struct DefaultHttpRetry {
    #[allow(dead_code)] // used once the real middleware impl lands
    config: RetryConfig,
}

impl DefaultHttpRetry {
    /// Construct from a resolved config.
    pub(crate) fn new(config: RetryConfig) -> Self {
        Self { config }
    }
}

impl HttpRetry for DefaultHttpRetry {
    fn describe(&self) -> &'static str {
        "swe_http_retry"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: DefaultHttpRetry::new
    #[test]
    fn test_new_constructs_and_stores_config() {
        let cfg = RetryConfig::swe_default().expect("baseline parses");
        let d = DefaultHttpRetry::new(cfg);
        let dbg = format!("{d:?}");
        assert!(dbg.contains("DefaultHttpRetry"), "debug output: {dbg}");
    }

    /// @covers: describe
    #[test]
    fn test_describe_returns_crate_name() {
        let cfg = RetryConfig::swe_default().expect("baseline parses");
        let d = DefaultHttpRetry::new(cfg);
        assert_eq!(d.describe(), "swe_http_retry");
    }
}
