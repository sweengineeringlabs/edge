//! Public type — the HTTP cache middleware layer.

use std::sync::Arc;

use moka::future::Cache;

use crate::api::cache_config::CacheConfig;

/// HTTP cache middleware. Attach to a
/// `reqwest_middleware::ClientBuilder` via `.with(layer)`.
///
/// Simple TTL-based cache — see `core::cache_layer` module
/// docs for the covered + uncovered RFC 7234 semantics.
pub struct CacheLayer {
    pub(crate) config: Arc<CacheConfig>,
    pub(crate) store:
        Cache<String, crate::core::cache_layer::CachedEntry>,
}

impl std::fmt::Debug for CacheLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheLayer")
            .field("default_ttl_seconds", &self.config.default_ttl_seconds)
            .field("max_entries", &self.config.max_entries)
            .field("respect_cache_control", &self.config.respect_cache_control)
            .field("cache_private", &self.config.cache_private)
            .finish()
    }
}
