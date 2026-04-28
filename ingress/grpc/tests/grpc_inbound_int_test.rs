//! Integration tests for the gRPC inbound domain.

use std::time::Duration;

use swe_edge_ingress_grpc::{
    GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode,
    GrpcInbound, GrpcInboundError, GrpcInboundResult, GrpcHealthCheck,
};
use futures::future::BoxFuture;

/// Stub that echoes back an empty gRPC response.
struct EchoGrpcHandler;

impl GrpcInbound for EchoGrpcHandler {
    fn handle_unary(&self, _request: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async {
            Ok(GrpcResponse { body: vec![0x00], metadata: GrpcMetadata::default() })
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async { Ok(GrpcHealthCheck::healthy()) })
    }
}

/// Stub that always returns an error.
struct FailingGrpcHandler;

impl GrpcInbound for FailingGrpcHandler {
    fn handle_unary(&self, _request: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async { Err(GrpcInboundError::Unavailable("service offline".into())) })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async { Ok(GrpcHealthCheck::unhealthy("service offline")) })
    }
}

#[tokio::test]
async fn test_grpc_inbound_handle_unary_returns_response() {
    let handler = EchoGrpcHandler;
    let req = GrpcRequest::new("pkg.Service/Method", vec![0x08, 0x01], Duration::from_secs(5));
    let resp = handler.handle_unary(req).await.unwrap();
    assert!(!resp.body.is_empty());
}

#[tokio::test]
async fn test_grpc_inbound_health_check_returns_healthy() {
    let handler = EchoGrpcHandler;
    let h = handler.health_check().await.unwrap();
    assert!(h.healthy);
    assert!(h.message.is_none());
}

#[tokio::test]
async fn test_grpc_inbound_unavailable_returns_error() {
    let handler = FailingGrpcHandler;
    let req = GrpcRequest::new("pkg.Service/Method", vec![], Duration::from_secs(5));
    let result = handler.handle_unary(req).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), GrpcInboundError::Unavailable(_)));
}

#[tokio::test]
async fn test_grpc_inbound_failing_health_check_is_unhealthy() {
    let handler = FailingGrpcHandler;
    let h = handler.health_check().await.unwrap();
    assert!(!h.healthy);
    assert!(h.message.is_some());
}

#[test]
fn test_grpc_metadata_default_has_empty_headers() {
    let m = GrpcMetadata::default();
    assert!(m.headers.is_empty());
}

#[test]
fn test_grpc_status_code_ok_equals_ok() {
    assert_eq!(GrpcStatusCode::Ok, GrpcStatusCode::Ok);
    assert_ne!(GrpcStatusCode::Ok, GrpcStatusCode::NotFound);
}

#[test]
fn test_grpc_request_holds_method_and_body() {
    let req = GrpcRequest::new("svc/Do", vec![1, 2, 3], Duration::from_secs(5));
    assert_eq!(req.method, "svc/Do");
    assert_eq!(req.body.len(), 3);
}
