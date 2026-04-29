//! Integration tests for [`MtlsAuthInterceptor`] — exercise the
//! crate's public facade exactly the way a consumer would, including
//! direct interaction with `swe_edge_ingress_grpc` types.

use std::collections::HashMap;
use std::time::Duration;

use swe_edge_egress_grpc_auth_mtls::{MtlsAuthConfig, MtlsAuthInterceptor};
use swe_edge_ingress_grpc::{
    GrpcInboundError, GrpcInboundInterceptor, GrpcMetadata, GrpcRequest, GrpcStatusCode,
    PEER_CERT_FINGERPRINT_SHA256, PEER_CN,
};

fn req_with_fingerprint_and_cn(cn: Option<&str>) -> GrpcRequest {
    let mut headers = HashMap::new();
    headers.insert(
        PEER_CERT_FINGERPRINT_SHA256.to_string(),
        "abc123".repeat(11),
    );
    if let Some(cn) = cn {
        headers.insert(PEER_CN.to_string(), cn.to_string());
    }
    GrpcRequest::new("/svc/M", vec![], Duration::from_secs(1))
        .with_metadata(GrpcMetadata { headers })
}

/// @covers: end-to-end — saf surface accepts a verified peer.
#[test]
fn mtls_auth_interceptor_struct_accepts_verified_peer_int_test() {
    let interceptor = MtlsAuthInterceptor::allow_any_verified_peer();
    let mut r = req_with_fingerprint_and_cn(Some("svc-a"));
    interceptor.before_dispatch(&mut r).expect("verified peer must pass");
}

/// @covers: saf — empty metadata = Unauthenticated through the chain.
#[test]
fn mtls_auth_interceptor_struct_rejects_plaintext_call_int_test() {
    let interceptor = MtlsAuthInterceptor::allow_any_verified_peer();
    let mut r = GrpcRequest::new("/svc/M", vec![], Duration::from_secs(1));
    match interceptor.before_dispatch(&mut r) {
        Err(GrpcInboundError::Status(GrpcStatusCode::Unauthenticated, _)) => {}
        other => panic!("expected Unauthenticated, got {other:?}"),
    }
}

/// @covers: saf — CN allowlist enforcement is reachable through the facade.
#[test]
fn mtls_auth_interceptor_struct_enforces_cn_allowlist_through_saf_int_test() {
    let interceptor =
        MtlsAuthInterceptor::from_config(MtlsAuthConfig::restrict_to_cns(["svc-a".into()]));
    let mut r = req_with_fingerprint_and_cn(Some("svc-evil"));
    match interceptor.before_dispatch(&mut r) {
        Err(GrpcInboundError::Status(GrpcStatusCode::PermissionDenied, _)) => {}
        other => panic!("expected PermissionDenied, got {other:?}"),
    }
}
