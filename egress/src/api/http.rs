//! HTTP egress trait — makes outbound HTTP requests.

use crate::api::error::EgressError;

/// Makes outbound HTTP requests to external services.
pub trait HttpClient: Send + Sync {
    /// A description of this HTTP client for diagnostics.
    fn describe(&self) -> &'static str;

    /// Verify the client can reach the configured base URL.
    fn health_check(&self) -> Result<(), EgressError>;
}
