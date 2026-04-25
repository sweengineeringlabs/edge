//! Integration tests for the gRPC outbound domain.

use swe_edge_egress_grpc::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};

/// @covers: GrpcRequest — struct construction.
#[test]
fn test_grpc_request_holds_method_and_body() {
    let req = GrpcRequest {
        method: "svc/Method".into(),
        body: vec![1, 2, 3],
        metadata: GrpcMetadata::default(),
    };
    assert_eq!(req.method, "svc/Method");
    assert_eq!(req.body, vec![1, 2, 3]);
}

/// @covers: GrpcMetadata::default — starts with empty headers.
#[test]
fn test_grpc_metadata_default_has_empty_headers() {
    let m = GrpcMetadata::default();
    assert!(m.headers.is_empty());
}

/// @covers: GrpcStatusCode — distinct variants.
#[test]
fn test_grpc_status_code_ok_is_distinct_from_internal() {
    assert_ne!(GrpcStatusCode::Ok, GrpcStatusCode::Internal);
}

/// @covers: GrpcResponse — struct construction.
#[test]
fn test_grpc_response_holds_body_bytes() {
    let resp = GrpcResponse { body: vec![0x08, 0x01], metadata: GrpcMetadata::default() };
    assert_eq!(resp.body, vec![0x08, 0x01]);
}
