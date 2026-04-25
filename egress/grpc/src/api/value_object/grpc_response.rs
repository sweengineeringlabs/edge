//! gRPC response envelope.

use super::grpc_metadata::GrpcMetadata;

/// A gRPC response envelope.
#[derive(Debug, Clone)]
pub struct GrpcResponse {
    pub body: Vec<u8>,
    pub metadata: GrpcMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_response_holds_body_bytes() {
        let resp = GrpcResponse { body: vec![0x08, 0x01], metadata: GrpcMetadata::default() };
        assert_eq!(resp.body, vec![0x08, 0x01]);
    }
}
