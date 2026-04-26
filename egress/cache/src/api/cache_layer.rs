//! Public type — the HTTP cache middleware layer.

use std::sync::Arc;

use moka::future::Cache;

use crate::api::cache_config::CacheConfig;
use crate::core::cached_entry::CachedEntry;

/// HTTP cache middleware. Attach to a
/// `reqwest_middleware::ClientBuilder` via `.with(layer)`.
///
/// Simple TTL-based cache — see `core::cache_layer` module
/// docs for the covered + uncovered RFC 7234 semantics.
pub struct CacheLayer {
    pub(crate) config: Arc<CacheConfig>,
    /// Primary store: `(method, url)` → Vec of CachedEntry variants
    /// (one variant per observed `Vary` combination). Wrapped in
    /// `Arc` so the moka value type stays cheap to clone on
    /// read-side copies.
    pub(crate) store: Cache<String, Arc<Vec<CachedEntry>>>,
    /// Client used for `stale-while-revalidate` background
    /// refreshes. The spawned refresh task cannot re-enter the
    /// middleware chain (`reqwest_middleware::Next<'a>` is
    /// non-`'static`), so SWR refreshes go out over this bare
    /// client — bypassing any other middleware in the chain.
    /// This is a documented limitation.
    pub(crate) swr_client: Arc<reqwest::Client>,
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
