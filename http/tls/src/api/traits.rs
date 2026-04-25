//! Primary trait re-export hub and trait definitions for `swe_edge_http_tls`.

pub(crate) type HttpTlsTrait = dyn crate::api::http_tls::HttpTls;

/// Groups types that are full TLS identity providers — they
/// implement [`HttpTls`][crate::api::http_tls::HttpTls] and can
/// be stored behind an `Arc<dyn HttpTls>` in the layer. Core
/// identity loaders declare `impl TlsIdentityProvider`.
pub(crate) trait TlsIdentityProvider: crate::api::http_tls::HttpTls {}

/// Public contract for applying a resolved TLS identity to a
/// [`reqwest::ClientBuilder`]. Implemented by [`TlsLayer`].
///
/// Consumers import this trait to call `layer.apply_to(builder)`.
pub trait TlsApplier {
    /// Augment `builder` with this layer's client identity.
    /// Returns the builder unchanged when the underlying provider
    /// is the `None` (pass-through) variant.
    fn apply_to(
        &self,
        builder: reqwest::ClientBuilder,
    ) -> Result<reqwest::ClientBuilder, crate::api::error::Error>;
}
