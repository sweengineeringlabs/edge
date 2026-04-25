//! Builder type declaration (rule 160 — public types live in api/).

use crate::api::rate_config::RateConfig;

/// Opaque builder for the rate-limiter middleware.
///
/// Construct via [`swe_http_rate::builder()`](crate::builder) or
/// [`Builder::with_config`]. Finalize with [`Builder::build`].
#[derive(Debug)]
pub struct Builder {
    /// The resolved rate-limiter policy.
    pub(crate) config: RateConfig,
}
