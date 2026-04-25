//! Default impl of [`HttpBreaker`](crate::api::http_breaker::HttpBreaker).
//!
//! Scaffold phase: holds a resolved [`BreakerConfig`](crate::api::breaker_config::BreakerConfig)
//! and answers `describe()`. Real middleware behavior lands
//! when the crate's `Middleware` impl is written — at that
//! point the strategy/policy state moves in here too.

use crate::api::breaker_config::BreakerConfig;
use crate::api::http_breaker::HttpBreaker;

/// Default HttpBreaker implementation. `pub(crate)` — consumers
/// never touch this type directly; they go through `saf::builder`.
#[derive(Debug)]
pub(crate) struct DefaultHttpBreaker {
    #[allow(dead_code)] // used once the real middleware impl lands
    config: BreakerConfig,
}

impl DefaultHttpBreaker {
    /// Construct from a resolved config.
    pub(crate) fn new(config: BreakerConfig) -> Self {
        Self { config }
    }
}

impl HttpBreaker for DefaultHttpBreaker {
    fn describe(&self) -> &'static str {
        "swe_http_breaker"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: describe
    #[test]
    fn test_describe_returns_crate_name() {
        let cfg = BreakerConfig::swe_default().expect("baseline parses");
        let d = DefaultHttpBreaker::new(cfg);
        assert_eq!(d.describe(), "swe_http_breaker");
    }
}
