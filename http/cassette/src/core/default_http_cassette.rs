//! Default impl of [`HttpCassette`](crate::api::http_cassette::HttpCassette).
//!
//! Scaffold phase: holds a resolved [`CassetteConfig`](crate::api::cassette_config::CassetteConfig)
//! and answers `describe()`. Real middleware behavior lands
//! when the crate's `Middleware` impl is written — at that
//! point the strategy/policy state moves in here too.

use crate::api::cassette_config::CassetteConfig;
use crate::api::http_cassette::HttpCassette;

/// Default HttpCassette implementation. `pub(crate)` — consumers
/// never touch this type directly; they go through `saf::builder`.
#[derive(Debug)]
pub(crate) struct DefaultHttpCassette {
    #[allow(dead_code)] // used once the real middleware impl lands
    config: CassetteConfig,
}

impl DefaultHttpCassette {
    /// Construct from a resolved config.
    pub(crate) fn new(config: CassetteConfig) -> Self {
        Self { config }
    }
}

impl HttpCassette for DefaultHttpCassette {
    fn describe(&self) -> &'static str {
        "swe_http_cassette"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: DefaultHttpCassette::new
    #[test]
    fn test_new_constructs_and_stores_config() {
        let cfg = CassetteConfig::swe_default().expect("baseline parses");
        let d = DefaultHttpCassette::new(cfg);
        let dbg = format!("{d:?}");
        assert!(dbg.contains("DefaultHttpCassette"), "debug output: {dbg}");
    }

    /// @covers: describe
    #[test]
    fn test_describe_returns_crate_name() {
        let cfg = CassetteConfig::swe_default().expect("baseline parses");
        let d = DefaultHttpCassette::new(cfg);
        assert_eq!(d.describe(), "swe_http_cassette");
    }
}
