//! Integration tests for `GrpcChannelConfig` and
//! `TonicGrpcClient::from_config`.
//!
//! Verifies the **fail-closed TLS-by-default invariant** at the
//! public-facade boundary.

use std::time::Duration;

use swe_edge_egress_grpc::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigError, GrpcOutbound, GrpcOutboundError,
    GrpcOutboundInterceptor, GrpcOutboundInterceptorChain, GrpcRequest, GrpcResponse,
    GrpcStatusCode, TonicGrpcClient, TraceContextInterceptor,
};

/// @covers: GrpcChannelConfig::default — tls_required is `true`.
/// Headline acceptance gate from issue #5.
#[test]
fn grpc_channel_config_struct_default_requires_tls_int_test() {
    let cfg = GrpcChannelConfig::default();
    assert!(cfg.tls_required, "TLS-by-default invariant must hold");
}

/// @covers: TonicGrpcClient::from_config — plaintext endpoint
/// rejected when `tls_required` is set.
#[test]
fn tonic_grpc_client_struct_from_config_rejects_plaintext_int_test() {
    let cfg = GrpcChannelConfig::new("http://localhost:50051");
    let result = TonicGrpcClient::from_config(&cfg);
    match result {
        Err(GrpcChannelConfigError::PlaintextRejected(endpoint)) => {
            assert!(endpoint.contains("localhost"), "endpoint in error: {endpoint}");
        }
        Ok(_) => panic!("must reject plaintext when tls_required=true"),
    }
}

/// @covers: TonicGrpcClient::from_config — plaintext accepted with opt-in.
#[test]
fn tonic_grpc_client_struct_from_config_accepts_plaintext_with_opt_in_int_test() {
    let cfg = GrpcChannelConfig::new("http://localhost:50051").allow_plaintext();
    assert!(TonicGrpcClient::from_config(&cfg).is_ok());
}

/// @covers: TonicGrpcClient::from_config — https endpoint accepted.
#[test]
fn tonic_grpc_client_struct_from_config_accepts_https_int_test() {
    let cfg = GrpcChannelConfig::new("https://example.com:443");
    assert!(TonicGrpcClient::from_config(&cfg).is_ok());
}

/// @covers: TonicGrpcClient::with_interceptors — accepts a chain.
#[test]
fn tonic_grpc_client_struct_with_interceptors_accepts_chain_int_test() {
    let chain = GrpcOutboundInterceptorChain::new()
        .push(std::sync::Arc::new(TraceContextInterceptor::pass_through()));
    let _client = TonicGrpcClient::new("http://localhost:0").with_interceptors(chain);
}

/// @covers: TonicGrpcClient — interceptor short-circuits before transport.
#[tokio::test]
async fn tonic_grpc_client_struct_interceptor_short_circuits_int_test() {
    struct Deny;
    impl GrpcOutboundInterceptor for Deny {
        fn before_call(&self, _: &mut GrpcRequest) -> Result<(), GrpcOutboundError> {
            Err(GrpcOutboundError::Status(
                GrpcStatusCode::PermissionDenied,
                "denied".into(),
            ))
        }
        fn after_call(&self, _: &mut GrpcResponse) -> Result<(), GrpcOutboundError> { Ok(()) }
    }

    let chain = GrpcOutboundInterceptorChain::new().push(std::sync::Arc::new(Deny));
    let client = TonicGrpcClient::new("http://127.0.0.1:1").with_interceptors(chain);
    let req = GrpcRequest::new("svc/Method", vec![1, 2, 3], Duration::from_secs(1));
    match client.call_unary(req).await {
        Err(GrpcOutboundError::Status(GrpcStatusCode::PermissionDenied, msg)) => {
            assert_eq!(msg, "denied");
        }
        other => panic!("expected PermissionDenied; got {other:?}"),
    }
}

/// @covers: TonicGrpcClient::with_compression — sets mode without panic.
#[test]
fn tonic_grpc_client_struct_with_compression_overrides_mode_int_test() {
    let _client = TonicGrpcClient::new("http://localhost:0")
        .with_compression(CompressionMode::Gzip);
}
