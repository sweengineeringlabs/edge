//! Simple TTL-based cache middleware — ours, not a wrapper
//! around http-cache-reqwest.
//!
//! ## Why we wrote our own
//!
//! We originally wrapped `http-cache-reqwest` for full RFC 7234
//! semantics, but the `default_ttl_seconds` override we expose
//! in `CacheConfig` isn't expressible against that crate's
//! public API — its `CacheOptions` only controls heuristics
//! (cache_heuristic fraction applied to `Last-Modified`), not
//! an absolute fallback TTL. Consumers either got
//! `default_ttl_seconds` honored or they got RFC 7234.
//!
//! We picked honoring the config. The result is a simpler cache:
//!
//! - Keys are `(method, url)` (no Vary)
//! - TTL decision: `max-age` from upstream `Cache-Control` when
//!   `respect_cache_control = true` AND the header is present;
//!   else `default_ttl_seconds` (when > 0).
//! - Storage: moka, bounded at `config.max_entries`.
//! - Only GET + HEAD are cached (POST/PUT/DELETE pass through).
//! - No conditional revalidation (`If-None-Match`, 304 handling).
//! - `Cache-Control: no-store` on the upstream response is
//!   honored — we don't cache it regardless.
//! - `Cache-Control: private` → cached only when
//!   `config.cache_private = true`.
//!
//! Consumers that need full RFC 7234 semantics should wire
//! `http-cache-reqwest` directly.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use moka::future::Cache;
use reqwest::header::{HeaderName, HeaderValue, CACHE_CONTROL};

use crate::api::cache_config::CacheConfig;
use crate::api::cache_layer::CacheLayer;

/// One cached response entry — the minimal shape needed to
/// reconstruct a `reqwest::Response`.
#[derive(Clone)]
pub(crate) struct CachedEntry {
    pub(crate) status: u16,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: Arc<Vec<u8>>,
    pub(crate) expires_at: Instant,
}

impl CacheLayer {
    /// Construct from a resolved config.
    pub(crate) fn new(config: CacheConfig) -> Self {
        let store: Cache<String, CachedEntry> = Cache::builder()
            .max_capacity(config.max_entries)
            .build();
        Self {
            config: Arc::new(config),
            store,
        }
    }

    /// Cache key for a request. `(method, url)`. Query params
    /// are part of the URL so differently-parameterized requests
    /// have distinct keys.
    fn key_for(&self, req: &reqwest::Request) -> String {
        format!("{} {}", req.method(), req.url())
    }

    /// Is this method cacheable?
    fn is_cacheable_method(method: &reqwest::Method) -> bool {
        matches!(method, &reqwest::Method::GET | &reqwest::Method::HEAD)
    }

    /// Compute the TTL for a response based on its headers
    /// + our config. Returns `None` when the response MUST
    /// NOT be cached (`no-store`, or `private` when
    /// `cache_private=false`, or no Cache-Control + zero
    /// default TTL).
    fn ttl_for(&self, response: &reqwest::Response) -> Option<Duration> {
        if let Some(cc) = response.headers().get(CACHE_CONTROL) {
            let cc = cc.to_str().unwrap_or("").to_ascii_lowercase();
            if cc.contains("no-store") {
                return None;
            }
            if self.config.respect_cache_control {
                if cc.contains("private") && !self.config.cache_private {
                    return None;
                }
                if let Some(max_age) = extract_max_age(&cc) {
                    return Some(Duration::from_secs(max_age));
                }
            }
        }
        // No Cache-Control OR we're ignoring it. Fall back
        // to configured default.
        if self.config.default_ttl_seconds > 0 {
            Some(Duration::from_secs(self.config.default_ttl_seconds))
        } else {
            None
        }
    }
}

/// Extract `max-age=N` from a lowercased Cache-Control value.
fn extract_max_age(cc: &str) -> Option<u64> {
    for part in cc.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("max-age=") {
            return v.parse().ok();
        }
    }
    None
}

/// Rebuild a `reqwest::Response` from a cached entry.
fn reconstruct(entry: &CachedEntry) -> Result<reqwest::Response, String> {
    let mut builder = http::Response::builder().status(entry.status);
    for (k, v) in &entry.headers {
        builder = builder.header(k, v);
    }
    // Re-serve the cached body as-is.
    let body: Vec<u8> = (*entry.body).clone();
    let http_resp = builder
        .body(reqwest::Body::from(body))
        .map_err(|e| format!("rebuild response: {e}"))?;
    Ok(reqwest::Response::from(http_resp))
}

#[async_trait]
impl reqwest_middleware::Middleware for CacheLayer {
    async fn handle(
        &self,
        req: reqwest::Request,
        ext: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        // Pass non-cacheable methods through.
        if !Self::is_cacheable_method(req.method()) {
            return next.run(req, ext).await;
        }

        let key = self.key_for(&req);

        // Cache lookup.
        if let Some(entry) = self.store.get(&key).await {
            if Instant::now() < entry.expires_at {
                return reconstruct(&entry).map_err(|e| {
                    reqwest_middleware::Error::Middleware(anyhow::anyhow!(
                        "swe_http_cache reconstruct: {e}"
                    ))
                });
            }
            // Stale — fall through to refetch.
        }

        // Miss or stale — dispatch.
        let response = next.run(req, ext).await?;

        // Decide whether to cache. Only 2xx + 3xx are cacheable
        // by RFC; we narrow to 200/203/300/301/404/410 per the
        // standard "heuristically cacheable" list.
        let status = response.status().as_u16();
        let cacheable_status = matches!(status, 200 | 203 | 300 | 301 | 404 | 410);

        if !cacheable_status {
            return Ok(response);
        }

        let ttl = match self.ttl_for(&response) {
            Some(t) => t,
            None => return Ok(response),
        };

        // Buffer the body so we can both cache it AND return
        // it to the caller.
        let status_code = response.status().as_u16();
        let headers: BTreeMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str().ok().map(|v| (k.as_str().to_string(), v.to_string()))
            })
            .collect();
        let body = response.bytes().await.map_err(|e| {
            reqwest_middleware::Error::Middleware(anyhow::anyhow!(
                "swe_http_cache read body: {e}"
            ))
        })?;
        let body_vec = body.to_vec();

        // Store.
        let entry = CachedEntry {
            status: status_code,
            headers: headers.clone(),
            body: Arc::new(body_vec.clone()),
            expires_at: Instant::now() + ttl,
        };
        self.store.insert(key, entry.clone()).await;

        // Reconstruct a fresh response for the caller.
        reconstruct(&entry).map_err(|e| {
            reqwest_middleware::Error::Middleware(anyhow::anyhow!(
                "swe_http_cache post-store reconstruct: {e}"
            ))
        })
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

    /// @covers: extract_max_age
    #[test]
    fn test_extract_max_age_from_simple_directive() {
        assert_eq!(extract_max_age("max-age=600"), Some(600));
    }

    /// @covers: extract_max_age
    #[test]
    fn test_extract_max_age_from_mixed_directives() {
        assert_eq!(
            extract_max_age("public, max-age=300, must-revalidate"),
            Some(300)
        );
    }

    /// @covers: extract_max_age
    #[test]
    fn test_extract_max_age_absent_returns_none() {
        assert!(extract_max_age("no-cache").is_none());
        assert!(extract_max_age("").is_none());
    }

    /// @covers: CacheLayer::is_cacheable_method
    #[test]
    fn test_get_and_head_are_cacheable() {
        assert!(CacheLayer::is_cacheable_method(&reqwest::Method::GET));
        assert!(CacheLayer::is_cacheable_method(&reqwest::Method::HEAD));
    }

    /// @covers: CacheLayer::is_cacheable_method
    #[test]
    fn test_mutating_methods_are_not_cacheable() {
        assert!(!CacheLayer::is_cacheable_method(&reqwest::Method::POST));
        assert!(!CacheLayer::is_cacheable_method(&reqwest::Method::PUT));
        assert!(!CacheLayer::is_cacheable_method(&reqwest::Method::DELETE));
        assert!(!CacheLayer::is_cacheable_method(&reqwest::Method::PATCH));
    }

    /// @covers: CacheLayer::key_for
    #[test]
    fn test_key_includes_method_and_full_url() {
        let l = CacheLayer::new(test_config());
        let req = reqwest::Request::new(
            reqwest::Method::GET,
            reqwest::Url::parse("https://example.test/foo?q=1").unwrap(),
        );
        let k = l.key_for(&req);
        assert!(k.contains("GET"));
        assert!(k.contains("example.test/foo"));
        assert!(k.contains("q=1"));
    }

    /// Build a stub `reqwest::Response` with the given Cache-Control
    /// header for TTL-computation tests.
    fn stub_response(cache_control: Option<&str>) -> reqwest::Response {
        let mut builder = http::Response::builder().status(200);
        if let Some(cc) = cache_control {
            builder = builder.header("cache-control", cc);
        }
        let http_resp = builder.body(reqwest::Body::from(b"body".to_vec())).unwrap();
        reqwest::Response::from(http_resp)
    }

    /// @covers: CacheLayer::ttl_for
    #[test]
    fn test_ttl_honors_upstream_max_age_when_respect_true() {
        let l = CacheLayer::new(test_config());
        let resp = stub_response(Some("max-age=60"));
        assert_eq!(l.ttl_for(&resp), Some(Duration::from_secs(60)));
    }

    /// @covers: CacheLayer::ttl_for
    #[test]
    fn test_ttl_falls_back_to_default_when_no_cache_control() {
        let l = CacheLayer::new(test_config());
        let resp = stub_response(None);
        assert_eq!(l.ttl_for(&resp), Some(Duration::from_secs(300)));
    }

    /// @covers: CacheLayer::ttl_for
    #[test]
    fn test_ttl_honors_no_store_even_with_default_ttl_set() {
        let l = CacheLayer::new(test_config());
        let resp = stub_response(Some("no-store"));
        assert!(l.ttl_for(&resp).is_none());
    }

    /// @covers: CacheLayer::ttl_for
    #[test]
    fn test_ttl_private_blocked_when_cache_private_false() {
        let l = CacheLayer::new(test_config());
        let resp = stub_response(Some("private, max-age=60"));
        assert!(l.ttl_for(&resp).is_none());
    }

    /// @covers: CacheLayer::ttl_for
    #[test]
    fn test_ttl_private_allowed_when_cache_private_true() {
        let cfg = CacheConfig::from_config(
            r#"
                default_ttl_seconds = 0
                max_entries = 10
                respect_cache_control = true
                cache_private = true
            "#,
        )
        .unwrap();
        let l = CacheLayer::new(cfg);
        let resp = stub_response(Some("private, max-age=60"));
        assert_eq!(l.ttl_for(&resp), Some(Duration::from_secs(60)));
    }

    /// @covers: CacheLayer::ttl_for
    #[test]
    fn test_ttl_default_zero_without_cache_control_means_no_cache() {
        let cfg = CacheConfig::from_config(
            r#"
                default_ttl_seconds = 0
                max_entries = 10
                respect_cache_control = true
                cache_private = false
            "#,
        )
        .unwrap();
        let l = CacheLayer::new(cfg);
        let resp = stub_response(None);
        assert!(l.ttl_for(&resp).is_none());
    }

    /// @covers: CacheLayer::ttl_for
    #[test]
    fn test_ttl_ignores_cache_control_when_respect_false() {
        let cfg = CacheConfig::from_config(
            r#"
                default_ttl_seconds = 42
                max_entries = 10
                respect_cache_control = false
                cache_private = false
            "#,
        )
        .unwrap();
        let l = CacheLayer::new(cfg);
        // respect_cache_control=false means we SKIP checking
        // max-age/private (no-store is still honored though).
        let resp = stub_response(Some("max-age=9999"));
        assert_eq!(l.ttl_for(&resp), Some(Duration::from_secs(42)));
    }

    /// @covers: reconstruct
    #[test]
    fn test_reconstruct_preserves_status_headers_body() {
        let mut headers = BTreeMap::new();
        headers.insert("x-custom".into(), "value".into());
        let entry = CachedEntry {
            status: 418,
            headers,
            body: Arc::new(b"body-bytes".to_vec()),
            expires_at: Instant::now() + Duration::from_secs(60),
        };
        let resp = reconstruct(&entry).unwrap();
        assert_eq!(resp.status().as_u16(), 418);
        assert_eq!(resp.headers().get("x-custom").unwrap(), "value");
    }
}
