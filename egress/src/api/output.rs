//! Outbound sink trait — writes data to a destination.

use crate::api::error::EgressError;

/// Writes outbound data to a sink (stdout, file, network, etc.).
pub trait OutboundSink: Send + Sync {
    /// Write raw bytes to this sink.
    fn write(&self, data: &[u8]) -> Result<(), EgressError>;
}
