//! Public type — the TLS layer consumers apply to a
//! `reqwest::ClientBuilder` before building.
//!
//! Rule 160: public types in api/. Impl blocks live in
//! `core::tls_layer`.

use std::sync::Arc;

use crate::api::http_tls::HttpTls;

/// TLS identity layer. Opaque handle — consumers get one from
/// `saf::builder()` → `Builder::build()` and apply it to a
/// `reqwest::ClientBuilder` via `apply_to(..)`.
///
/// ```ignore
/// let tls = swe_http_tls::builder()?.with_config(cfg).build()?;
/// let client = tls.apply_to(reqwest::Client::builder())?.build()?;
/// ```
pub struct TlsLayer {
    pub(crate) provider: Arc<dyn HttpTls>,
}

impl std::fmt::Debug for TlsLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsLayer")
            .field("provider", &self.provider.describe())
            .finish()
    }
}
