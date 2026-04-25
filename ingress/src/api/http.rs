//! HTTP inbound trait — receives HTTP requests from upstream callers.

use crate::api::error::IngressError;

/// Receives and deserialises inbound HTTP requests.
pub trait HttpInbound: Send + Sync {
    /// A description of this HTTP inbound adapter for diagnostics.
    fn describe(&self) -> &'static str;

    /// Verify the adapter is reachable and accepting requests.
    fn health_check(&self) -> Result<(), IngressError>;
}
