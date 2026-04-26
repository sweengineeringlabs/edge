//! gRPC request envelope.

use super::grpc_metadata::GrpcMetadata;

/// A gRPC request envelope.
#[derive(Debug, Clone)]
pub struct GrpcRequest {
    pub method: String,
    pub body: Vec<u8>,
    pub metadata: GrpcMetadata,
}

impl GrpcRequest {
    /// Create a new request for `method` with a raw protobuf `body`.
    pub fn new(method: impl Into<String>, body: Vec<u8>) -> Self {
        Self { method: method.into(), body, metadata: GrpcMetadata::default() }
    }

    /// Override the entire metadata block.
    pub fn with_metadata(mut self, metadata: GrpcMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a single metadata header key/value pair.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.headers.insert(name.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_request_holds_method_and_body() {
        let req = GrpcRequest { method: "svc/Method".into(), body: vec![1, 2], metadata: GrpcMetadata::default() };
        assert_eq!(req.method, "svc/Method");
        assert_eq!(req.body, vec![1, 2]);
    }

    #[test]
    fn test_new_creates_request_with_empty_metadata() {
        let req = GrpcRequest::new("pkg.Svc/Method", vec![0xAB]);
        assert_eq!(req.method, "pkg.Svc/Method");
        assert_eq!(req.body, vec![0xAB]);
        assert!(req.metadata.headers.is_empty());
    }

    #[test]
    fn test_with_header_inserts_single_metadata_entry() {
        let req = GrpcRequest::new("svc/M", vec![]).with_header("authorization", "Bearer tok");
        assert_eq!(req.metadata.headers.get("authorization").map(String::as_str), Some("Bearer tok"));
    }

    #[test]
    fn test_with_metadata_replaces_metadata_entirely() {
        let meta = GrpcMetadata { headers: [("k".to_string(), "v".to_string())].into_iter().collect() };
        let req = GrpcRequest::new("svc/M", vec![]).with_metadata(meta);
        assert_eq!(req.metadata.headers.get("k").map(String::as_str), Some("v"));
    }
}
