//! Public type — the HTTP cache middleware layer.

use std::sync::Arc;

use crate::api::cache_config::CacheConfig;

/// HTTP cache middleware. Attach to a
/// `reqwest_middleware::ClientBuilder` via `.with(layer)`.
///
/// Honors RFC 7234 cache semantics via the underlying
/// `http-cache-reqwest` crate. The policy we apply on top:
/// max_entries (moka eviction), CacheMode selection based on
/// `respect_cache_control`.
pub struct CacheLayer {
    pub(crate) config: Arc<CacheConfig>,
    pub(crate) inner: http_cache_reqwest::Cache<http_cache_reqwest::MokaManager>,
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
