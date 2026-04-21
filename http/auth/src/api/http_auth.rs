//! Primary trait for the auth crate (rule 153).
//!
//! `pub(crate)` — consumers never implement this trait. Plug-in
//! extension happens through new `AuthConfig` variants, not
//! external trait impls. The middleware in
//! `core::auth_middleware` holds an `Arc<dyn HttpAuth>` and
//! awaits `process` on each request.

use async_trait::async_trait;

use crate::api::error::Error;

/// Auth processor contract. Every middleware layer this crate
/// produces implements it.
#[async_trait]
pub(crate) trait HttpAuth: Send + Sync + std::fmt::Debug {
    /// Identify this processor in log / trace output.
    fn describe(&self) -> &'static str;

    /// Apply the configured auth policy to an outbound request.
    ///
    /// Async so strategies that need pre-request setup (Digest
    /// fetching a fresh nonce via side-channel) fit the same
    /// shape as synchronous schemes. For the sync strategies
    /// (Bearer/Basic/Header/Noop/AwsSigV4), the async overhead
    /// is trivial — an already-ready future.
    async fn process(&self, req: &mut reqwest::Request) -> Result<(), Error>;
}
