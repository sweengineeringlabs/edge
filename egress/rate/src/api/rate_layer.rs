//! Public type — the rate-limiter middleware layer.

use std::sync::Arc;

use moka::future::Cache;

use crate::api::rate_config::RateConfig;

/// Rate-limiter middleware. Attach to a
/// `reqwest_middleware::ClientBuilder` via `.with(layer)`.
pub struct RateLayer {
    pub(crate) config: Arc<RateConfig>,
    /// Per-host token buckets, keyed by authority
    /// (host:port). When `config.per_host = false`, a single
    /// bucket keyed by the empty string serves all requests.
    pub(crate) buckets: Cache<String, Arc<tokio::sync::Mutex<crate::core::token_bucket::TokenBucket>>>,
}

impl std::fmt::Debug for RateLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLayer")
            .field("tokens_per_second", &self.config.tokens_per_second)
            .field("burst_capacity", &self.config.burst_capacity)
            .field("per_host", &self.config.per_host)
            .finish()
    }
}
