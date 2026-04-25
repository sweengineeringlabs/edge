//! Builder type declaration (rule 160 — public types live in api/).

use crate::api::cache_config::CacheConfig;

/// Opaque builder for the HTTP cache middleware.
///
/// Construct via [`swe_http_cache::builder()`](crate::builder) or
/// [`Builder::with_config`]. Finalize with [`Builder::build`].
#[derive(Debug)]
pub struct Builder {
    /// The resolved cache policy.
    pub(crate) config: CacheConfig,
}
