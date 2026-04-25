//! Impl blocks for [`RateLayer`] — constructor +
//! [`reqwest_middleware::Middleware`] impl.

use std::sync::Arc;

use async_trait::async_trait;
use moka::future::Cache;
use tokio::sync::Mutex;

use crate::api::rate_config::RateConfig;
use crate::api::rate_layer::RateLayer;

use super::token_bucket::TokenBucket;

const MAX_TRACKED_HOSTS: u64 = 10_000;

/// When `per_host = false`, every request routes to the same
/// bucket keyed by this sentinel.
const GLOBAL_KEY: &str = "__global__";

impl RateLayer {
    pub(crate) fn new(config: RateConfig) -> Self {
        let buckets: Cache<String, Arc<Mutex<TokenBucket>>> =
            Cache::builder().max_capacity(MAX_TRACKED_HOSTS).build();
        Self {
            config: Arc::new(config),
            buckets,
        }
    }

    /// Bucket key for a given request.
    fn key_for(&self, req: &reqwest::Request) -> String {
        if !self.config.per_host {
            return GLOBAL_KEY.to_string();
        }
        match req.url().host_str() {
            Some(host) => match req.url().port() {
                Some(port) => format!("{host}:{port}"),
                None => host.to_string(),
            },
            None => "__hostless__".to_string(),
        }
    }

    /// Get-or-insert per-key bucket.
    async fn bucket(&self, key: &str) -> Arc<Mutex<TokenBucket>> {
        let cfg = self.config.clone();
        self.buckets
            .get_with(key.to_string(), async move {
                Arc::new(Mutex::new(TokenBucket::full(&cfg)))
            })
            .await
    }
}

#[async_trait]
impl reqwest_middleware::Middleware for RateLayer {
    async fn handle(
        &self,
        req: reqwest::Request,
        ext: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let key = self.key_for(&req);
        let bucket = self.bucket(&key).await;

        // Acquire loop — try_acquire, if empty sleep for the
        // indicated wait, retry. Holding the mutex across the
        // sleep keeps the ordering fair (first waiter wakes
        // first when tokens become available).
        //
        // Production note: in extreme contention the lock
        // becomes a queue. For throughput-critical workloads
        // this is usually what you want — a FIFO on the
        // limiter. If strict fairness matters, consider the
        // `governor` crate instead.
        loop {
            let wait = {
                let mut b = bucket.lock().await;
                match b.try_acquire(&self.config) {
                    Ok(()) => break, // token acquired, drop lock
                    Err(w) => w,
                }
            };
            tokio::time::sleep(wait).await;
        }

        next.run(req, ext).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RateConfig {
        RateConfig::from_config(
            r#"
                tokens_per_second = 10
                burst_capacity = 20
                per_host = true
            "#,
        )
        .unwrap()
    }

    fn global_config() -> RateConfig {
        RateConfig::from_config(
            r#"
                tokens_per_second = 10
                burst_capacity = 20
                per_host = false
            "#,
        )
        .unwrap()
    }

    fn stub_req(url: &str) -> reqwest::Request {
        reqwest::Request::new(
            reqwest::Method::GET,
            reqwest::Url::parse(url).unwrap(),
        )
    }

    /// @covers: RateLayer::new
    #[test]
    fn test_new_constructs_with_bucket_cache() {
        let _l = RateLayer::new(test_config());
    }

    /// @covers: RateLayer::key_for
    #[test]
    fn test_key_for_per_host_returns_authority() {
        let l = RateLayer::new(test_config());
        let k = l.key_for(&stub_req("http://example.test:8080/path"));
        assert_eq!(k, "example.test:8080");
    }

    /// @covers: RateLayer::key_for
    #[test]
    fn test_key_for_per_host_omits_default_port() {
        let l = RateLayer::new(test_config());
        let k = l.key_for(&stub_req("http://example.test/"));
        assert_eq!(k, "example.test");
    }

    /// @covers: RateLayer::key_for
    #[test]
    fn test_key_for_global_mode_same_for_all_hosts() {
        let l = RateLayer::new(global_config());
        let k1 = l.key_for(&stub_req("http://a.test/"));
        let k2 = l.key_for(&stub_req("http://b.test/"));
        assert_eq!(k1, k2);
    }

    /// @covers: RateLayer::bucket
    #[tokio::test]
    async fn test_bucket_shared_across_calls_for_same_key() {
        let l = RateLayer::new(test_config());
        let a = l.bucket("example.test").await;
        let b = l.bucket("example.test").await;
        assert!(Arc::ptr_eq(&a, &b));
    }

    /// @covers: RateLayer::bucket
    #[tokio::test]
    async fn test_bucket_distinct_for_different_keys() {
        let l = RateLayer::new(test_config());
        let a = l.bucket("a.test").await;
        let b = l.bucket("b.test").await;
        assert!(!Arc::ptr_eq(&a, &b));
    }
}
