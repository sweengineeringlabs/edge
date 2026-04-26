//! Primary trait re-export hub and trait definitions for `swe_edge_egress_tls`.

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
