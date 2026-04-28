//! Compression negotiation modes for gRPC channels.

use serde::{Deserialize, Serialize};

/// Wire-level compression scheme for gRPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CompressionMode {
    /// No compression.
    #[default]
    None,
    /// gzip compression.
    Gzip,
    /// zstandard compression.
    Zstd,
}

impl CompressionMode {
    /// `grpc-encoding` header value, or `None` for identity-only.
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

    /// @covers: CompressionMode::header_value — None returns None.
    #[test]
    fn test_header_value_for_none_returns_none() {
        assert_eq!(CompressionMode::None.header_value(), None);
    }

    /// @covers: CompressionMode::header_value — canonical names.
    #[test]
    fn test_header_value_for_gzip_and_zstd_uses_canonical_names() {
        assert_eq!(CompressionMode::Gzip.header_value(), Some("gzip"));
        assert_eq!(CompressionMode::Zstd.header_value(), Some("zstd"));
    }
}
