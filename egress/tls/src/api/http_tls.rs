//! Primary trait for the tls crate (rule 153).
//!
//! `pub(crate)` — consumers never implement. Plug-in happens
//! through new `TlsConfig` variants.

use crate::api::error::Error;

/// TLS identity provider contract. Produces a
/// [`reqwest::Identity`] on demand — the factory resolves the
/// config once at build time, then calls `identity()` to hand
/// the concrete value to [`reqwest::ClientBuilder::identity`].
pub trait HttpTls: Send + Sync + std::fmt::Debug {
    /// Identify this provider in log / trace output.
    fn describe(&self) -> &'static str;

    /// Produce the identity (or None for pass-through).
    /// Called once when attaching to a ClientBuilder.
    fn identity(&self) -> Result<Option<reqwest::Identity>, Error>;
}
