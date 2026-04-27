//! gRPC request envelope.

use super::grpc_metadata::GrpcMetadata;

/// A gRPC request envelope.
#[derive(Debug, Clone)]
pub struct GrpcRequest {
    /// Fully-qualified gRPC method path (e.g. `"/pkg.Service/Method"`).
    pub method: String,
    /// Raw protobuf-encoded request bytes.
    pub body: Vec<u8>,
    /// Request metadata (headers / trailers).
    pub metadata: GrpcMetadata,
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
}
