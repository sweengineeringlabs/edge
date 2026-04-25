//! Primary trait re-export hub and trait definitions for `swe_edge_http_rate`.

pub(crate) type HttpRateTrait = dyn crate::api::http_rate::HttpRate;

/// Contract for token-bucket rate limiters. Implementations
/// track per-host (or global) token counts and report whether
/// the current request may proceed or must wait.
pub(crate) trait RateBucketOps {
    /// Try to consume one token. Returns `Ok(())` if a token was
    /// available; `Err(wait)` is the minimum delay before a
    /// token will be ready.
    fn try_consume(
        &mut self,
        config: &crate::api::rate_config::RateConfig,
    ) -> Result<(), std::time::Duration>;
}
