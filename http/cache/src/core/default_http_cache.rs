//! Default impl of [`HttpCache`](crate::api::http_cache::HttpCache).
//!
//! Scaffold phase: holds a resolved [`CacheConfig`](crate::api::cache_config::CacheConfig)
//! and answers `describe()`. Real middleware behavior lands
//! when the crate's `Middleware` impl is written — at that
//! point the strategy/policy state moves in here too.

use crate::api::cache_config::CacheConfig;
use crate::api::http_cache::HttpCache;

/// Default HttpCache implementation. `pub(crate)` — consumers
/// never touch this type directly; they go through `saf::builder`.
#[derive(Debug)]
pub(crate) struct DefaultHttpCache {
    #[allow(dead_code)] // used once the real middleware impl lands
    config: CacheConfig,
}

impl DefaultHttpCache {
    /// Construct from a resolved config.
    pub(crate) fn new(config: CacheConfig) -> Self {
        Self { config }
    }
}

impl HttpCache for DefaultHttpCache {
    fn describe(&self) -> &'static str {
        "swe_http_cache"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: describe
    #[test]
    fn test_describe_returns_crate_name() {
        let cfg = CacheConfig::swe_default().expect("baseline parses");
        let d = DefaultHttpCache::new(cfg);
        assert_eq!(d.describe(), "swe_http_cache");
    }
}
