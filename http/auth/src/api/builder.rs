//! Builder type declaration (rule 160 — public types live in api/).
//!
//! The impl blocks for `Builder` live in `saf::builder` per rule
//! 154 (impls belong in the layer that owns the logic). This file
//! declares the public struct shape so the type is anchored in api/.

use crate::api::auth_config::AuthConfig;
use crate::api::credential_resolver::CredentialResolver;

/// Opaque builder for the auth middleware.
///
/// Construct via [`swe_edge_http_auth::builder()`](crate::builder) or
/// [`Builder::with_config`]. Finalize with [`Builder::build`].
pub struct Builder {
    /// The resolved auth policy.
    pub(crate) config: AuthConfig,
    /// Credential resolver (default: reads from process env vars).
    pub(crate) resolver: Box<dyn CredentialResolver>,
}
