//! gRPC request/response metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata for a gRPC request/response (headers).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GrpcMetadata {
    pub headers: HashMap<String, String>,
}

impl GrpcMetadata {
    /// Add a single header entry, returning the modified metadata.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_metadata_default_has_empty_headers() {
        let m = GrpcMetadata::default();
        assert!(m.headers.is_empty());
    }

    #[test]
    fn test_with_header_inserts_entry_into_headers_map() {
        let m = GrpcMetadata::default().with_header("x-request-id", "req-1");
        assert_eq!(m.headers.get("x-request-id").map(String::as_str), Some("req-1"));
    }

    #[test]
    fn test_with_header_chaining_accumulates_entries() {
        let m = GrpcMetadata::default()
            .with_header("a", "1")
            .with_header("b", "2");
        assert_eq!(m.headers.len(), 2);
    }
}
