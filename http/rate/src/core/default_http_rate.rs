//! Default impl of [`HttpRate`](crate::api::http_rate::HttpRate).
//!
//! Scaffold phase: holds a resolved [`RateConfig`](crate::api::rate_config::RateConfig)
//! and answers `describe()`. Real middleware behavior lands
//! when the crate's `Middleware` impl is written — at that
//! point the strategy/policy state moves in here too.

use crate::api::rate_config::RateConfig;
use crate::api::http_rate::HttpRate;

/// Default HttpRate implementation. `pub(crate)` — consumers
/// never touch this type directly; they go through `saf::builder`.
#[derive(Debug)]
pub(crate) struct DefaultHttpRate {
    #[allow(dead_code)] // used once the real middleware impl lands
    config: RateConfig,
}

impl DefaultHttpRate {
    /// Construct from a resolved config.
    pub(crate) fn new(config: RateConfig) -> Self {
        Self { config }
    }
}

impl HttpRate for DefaultHttpRate {
    fn describe(&self) -> &'static str {
        "swe_http_rate"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: describe
    #[test]
    fn test_describe_returns_crate_name() {
        let cfg = RateConfig::swe_default().expect("baseline parses");
        let d = DefaultHttpRate::new(cfg);
        assert_eq!(d.describe(), "swe_http_rate");
    }
}
