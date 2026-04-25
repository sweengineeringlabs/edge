//! Public type — the circuit breaker middleware layer.

use std::sync::Arc;

use moka::future::Cache;

use crate::api::breaker_config::BreakerConfig;

/// Circuit breaker middleware. Attach to a
/// `reqwest_middleware::ClientBuilder` via `.with(layer)`.
pub struct BreakerLayer {
    pub(crate) config: Arc<BreakerConfig>,
    /// Per-host state, keyed by the URL's authority
    /// (host:port). `moka::future::Cache` gives us async-safe
    /// concurrent access with background expiration of
    /// long-idle entries.
    pub(crate) state: Cache<String, Arc<tokio::sync::Mutex<crate::core::host_breaker::HostBreaker>>>,
}

impl std::fmt::Debug for BreakerLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BreakerLayer")
            .field("failure_threshold", &self.config.failure_threshold)
            .field("half_open_after_seconds", &self.config.half_open_after_seconds)
            .field("reset_after_successes", &self.config.reset_after_successes)
            .finish()
    }
}
