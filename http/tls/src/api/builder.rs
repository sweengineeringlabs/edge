//! Builder type declaration (rule 160 — public types live in api/).

use crate::api::tls_config::TlsConfig;

/// Opaque builder for the TLS identity layer.
///
/// Construct via [`swe_edge_http_tls::builder()`](crate::builder) or
/// [`Builder::with_config`]. Finalize with [`Builder::build`].
pub struct Builder {
    /// The resolved TLS identity policy.
    pub(crate) config: TlsConfig,
}

impl std::fmt::Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("config", &self.config)
            .finish()
    }
}
