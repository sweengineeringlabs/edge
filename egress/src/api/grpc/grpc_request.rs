//! gRPC request envelope.

use super::grpc_metadata::GrpcMetadata;

/// A gRPC request envelope.
#[derive(Debug, Clone)]
pub struct GrpcRequest {
    pub method: String,
    pub body: Vec<u8>,
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
