//! Impl blocks for [`CacheLayer`] — constructor +
//! [`reqwest_middleware::Middleware`] impl.

use std::sync::Arc;

use async_trait::async_trait;
use http_cache_reqwest::{Cache, CacheMode, HttpCache, HttpCacheOptions, MokaManager};

use crate::api::cache_config::CacheConfig;
use crate::api::cache_layer::CacheLayer;

impl CacheLayer {
    /// Construct from a resolved config.
    ///
    /// The resulting middleware uses `http-cache-reqwest`'s
    /// RFC 7234 engine with a moka-backed storage manager
    /// bounded at `config.max_entries`.
    pub(crate) fn new(config: CacheConfig) -> Self {
        let store: moka::future::Cache<String, Arc<Vec<u8>>> =
            moka::future::Cache::builder()
                .max_capacity(config.max_entries)
                .build();
        let manager = MokaManager::new(store);

        // Mode selection:
        // - respect_cache_control=true → CacheMode::Default
        //   (honors Cache-Control: no-store, max-age, etc.)
        // - respect_cache_control=false → CacheMode::IgnoreRules
        //   (caches everything regardless of upstream hints;
        //   use sparingly — can cache things the upstream
        //   wanted kept fresh)
        let mode = if config.respect_cache_control {
            CacheMode::Default
        } else {
            CacheMode::IgnoreRules
        };

        // NOTE on config fields not mapped to http-cache-reqwest:
        // - `default_ttl_seconds`: http-cache-reqwest derives TTL
        //   from the upstream Cache-Control max-age. Overriding
        //   that requires a custom CachePolicy impl — follow-up
        //   when a consumer actually needs it.
        // - `cache_private`: http-cache-reqwest uses CachePolicy
        //   shared=true by default (treats us as a shared
        //   cache, so Cache-Control: private isn't cached).
        //   Flipping to shared=false would require custom
        //   options; keep default semantics for now.
        let inner = Cache(HttpCache {
            mode,
            manager,
            options: HttpCacheOptions::default(),
        });

        Self {
            config: Arc::new(config),
            inner,
        }
    }
}

#[async_trait]
impl reqwest_middleware::Middleware for CacheLayer {
    async fn handle(
        &self,
        req: reqwest::Request,
        ext: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        <_ as reqwest_middleware::Middleware>::handle(&self.inner, req, ext, next).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CacheConfig {
        CacheConfig::from_config(
            r#"
                default_ttl_seconds = 300
                max_entries = 100
                respect_cache_control = true
                cache_private = false
            "#,
        )
        .unwrap()
    }

    /// @covers: CacheLayer::new
    #[test]
    fn test_new_constructs_with_respect_cache_control_default() {
        let _l = CacheLayer::new(test_config());
    }

    /// @covers: CacheLayer::new
    #[test]
    fn test_new_constructs_with_ignore_rules_mode() {
        let cfg = CacheConfig::from_config(
            r#"
                default_ttl_seconds = 60
                max_entries = 10
                respect_cache_control = false
                cache_private = false
            "#,
        )
        .unwrap();
        let _l = CacheLayer::new(cfg);
    }

    /// @covers: CacheLayer (Debug impl)
    #[test]
    fn test_debug_impl_shows_config_fields() {
        let l = CacheLayer::new(test_config());
        let s = format!("{l:?}");
        assert!(s.contains("CacheLayer"));
        assert!(s.contains("max_entries"));
        assert!(s.contains("default_ttl_seconds"));
    }
}
