//! Builder type declaration (rule 160 — public types live in api/).

use crate::api::breaker_config::BreakerConfig;

/// Opaque builder for the circuit-breaker middleware.
///
/// Construct via [`swe_http_breaker::builder()`](crate::builder) or
/// [`Builder::with_config`]. Finalize with [`Builder::build`].
#[derive(Debug)]
pub struct Builder {
    /// The resolved breaker policy.
    pub(crate) config: BreakerConfig,
}
