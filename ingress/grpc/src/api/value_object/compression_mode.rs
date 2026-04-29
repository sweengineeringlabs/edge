//! Compression negotiation modes for gRPC servers.

use serde::{Deserialize, Serialize};

/// Wire-level compression scheme accepted by the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CompressionMode {
    /// No compression accepted — identity only.
    #[default]
    None,
    /// gzip compression accepted.
    Gzip,
    /// zstandard compression accepted.
    Zstd,
}

impl CompressionMode {
    /// `grpc-accept-encoding` advertised value, or `None` for identity-only.
    pub fn header_value(self) -> Option<&'static str> {
        match self {
            CompressionMode::None => None,
            CompressionMode::Gzip => Some("gzip"),
            CompressionMode::Zstd => Some("zstd"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: CompressionMode::default — defaults to None.
    #[test]
    fn test_default_is_none_compression() {
        assert_eq!(CompressionMode::default(), CompressionMode::None);
    }

    /// @covers: CompressionMode::header_value — canonical names.
    #[test]
    fn test_header_value_uses_canonical_grpc_identifiers() {
        assert_eq!(CompressionMode::None.header_value(), None);
        assert_eq!(CompressionMode::Gzip.header_value(), Some("gzip"));
        assert_eq!(CompressionMode::Zstd.header_value(), Some("zstd"));
    }
}
