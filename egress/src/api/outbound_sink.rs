//! Outbound sink trait — writes data to a destination.

use crate::api::egress_error::EgressError;

/// Writes outbound data to a sink (stdout, file, network, etc.).
#[allow(dead_code)]
pub trait OutboundSink: Send + Sync {
    /// Write raw bytes to this sink.
    fn write(&self, data: &[u8]) -> Result<(), EgressError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DevNull;
    impl OutboundSink for DevNull {
        fn write(&self, _data: &[u8]) -> Result<(), EgressError> { Ok(()) }
    }

    #[test]
    fn test_outbound_sink_write_succeeds() {
        assert!(DevNull.write(b"hello").is_ok());
    }

    #[test]
    fn test_outbound_sink_write_empty_bytes_succeeds() {
        assert!(DevNull.write(&[]).is_ok());
    }
}
