//! Public type — the middleware layer consumers plug into
//! `reqwest_middleware::ClientBuilder::with(..)`.
//!
//! Rule 160: all public types live in api/. The actual `impl`
//! blocks (constructor + reqwest_middleware::Middleware impl)
//! live in `core::auth_middleware` to keep api/ logic-free per
//! rule 114 (api must not import from core).
//!
//! Consumers hold this type as an opaque handle — the single
//! pub(crate) field keeps internals hidden while still
//! satisfying rule 160 (public type declared here).

use std::sync::Arc;

use crate::api::http_auth::HttpAuth;

/// reqwest-middleware layer that applies the configured auth
/// policy to every outbound request.
///
/// Construct via `saf::builder()` → `Builder::build()`. Use:
///
/// ```ignore
/// let mw = swe_edge_http_auth::builder()?.build()?;
/// let client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
///     .with(mw)
///     .build();
/// ```
pub struct AuthMiddleware {
    /// The resolved `HttpAuth` processor behind the
    /// middleware. `pub(crate)` so core/ can construct it and
    /// the `Middleware` impl can read it; external callers see
    /// an opaque struct.
    pub(crate) processor: Arc<dyn HttpAuth>,
}

impl std::fmt::Debug for AuthMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthMiddleware")
            .field("processor", &self.processor.describe())
            .finish()
    }
}
