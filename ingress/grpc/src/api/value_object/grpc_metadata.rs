//! gRPC request/response metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata for a gRPC request/response (headers).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GrpcMetadata {
    /// Metadata key/value pairs (request headers or response trailers).
    pub headers: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_metadata_default_has_empty_headers() {
        let m = GrpcMetadata::default();
        assert!(m.headers.is_empty());
    }
}
