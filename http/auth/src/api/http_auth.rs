//! Primary trait for the auth crate (rule 153).
//!
//! `pub(crate)` — consumers never implement this trait. Plug-in
//! extension happens through new `AuthConfig` variants, not
//! external trait impls. The middleware in `core::auth_middleware`
//! holds an `Arc<dyn HttpAuth>` and delegates on each request.

use crate::api::error::Error;

/// Auth processor contract. Every middleware layer this crate
/// produces implements it.
pub(crate) trait HttpAuth: Send + Sync + std::fmt::Debug {
    /// Identify this processor in log / trace output.
    fn describe(&self) -> &'static str;

    /// Apply the configured auth policy to an outbound request.
    /// Called once per outbound call by
    /// [`AuthMiddleware`](crate::api::auth_middleware::AuthMiddleware).
    fn process(&self, req: &mut reqwest::Request) -> Result<(), Error>;
}
