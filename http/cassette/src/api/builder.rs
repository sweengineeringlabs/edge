//! Builder type declaration (rule 160 — public types live in api/).

use crate::api::cassette_config::CassetteConfig;

/// Opaque builder for the cassette (VCR) middleware.
///
/// Construct via [`swe_edge_http_cassette::builder()`](crate::builder) or
/// [`Builder::with_config`]. Finalize with [`Builder::build`].
#[derive(Debug)]
pub struct Builder {
    /// The resolved cassette policy.
    pub(crate) config: CassetteConfig,
    /// Cassette name (maps to a `<name>.yaml` fixture file).
    pub(crate) cassette_name: String,
}
