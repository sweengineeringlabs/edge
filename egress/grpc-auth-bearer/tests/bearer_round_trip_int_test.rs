//! End-to-end integration tests for the bearer interceptor pair.
//!
//! Runs the outbound interceptor's mint step, then immediately
//! feeds the produced `authorization` header into the inbound
//! interceptor's validation step.  This exercises both halves of
//! the contract on the same crate's public surface.

use std::time::Duration;

use swe_edge_egress_grpc::{
    GrpcOutboundInterceptor, GrpcRequest as OutReq,
};
use swe_edge_egress_grpc_auth_bearer::{
    BearerInboundConfig, BearerInboundInterceptor, BearerOutboundConfig, BearerOutboundInterceptor,
    BearerSecret, AUTHORIZATION_HEADER, EXTRACTED_BEARER_SUBJECT,
};
use swe_edge_ingress_grpc::{
    GrpcInboundError, GrpcInboundInterceptor, GrpcMetadata, GrpcRequest as InReq, GrpcStatusCode,
};

const SECRET: &[u8] = b"the-quick-brown-fox-jumps-over-32-bytes!";

fn outbound_cfg() -> BearerOutboundConfig {
    BearerOutboundConfig {
        secret: BearerSecret::Hs256 { secret: SECRET.to_vec() },
        issuer: "svc-a".into(),
        audience: "svc-b".into(),
        subject: "alice".into(),
        lifetime_seconds: 60,
    }
}

fn inbound_cfg() -> BearerInboundConfig {
    BearerInboundConfig {
        secret: BearerSecret::Hs256 { secret: SECRET.to_vec() },
        expected_issuer: "svc-a".into(),
        expected_audience: "svc-b".into(),
        leeway_seconds: 0,
    }
}

/// @covers: outbound mints → inbound validates → subject republished.
#[test]
fn bearer_outbound_to_inbound_round_trip_publishes_subject_int_test() {
    let outbound = BearerOutboundInterceptor::from_config(outbound_cfg());
    let inbound  = BearerInboundInterceptor::from_config(inbound_cfg());

    let mut out_req = OutReq::new("/svc/M", vec![1, 2, 3], Duration::from_secs(1));
    outbound.before_call(&mut out_req).expect("mint");
    let auth = out_req
        .metadata
        .headers
        .get(AUTHORIZATION_HEADER)
        .cloned()
        .expect("outbound must inject Authorization");
    assert!(auth.starts_with("Bearer "));

    // Move into inbound shape and run the validator.
    let mut in_headers = std::collections::HashMap::new();
    in_headers.insert(AUTHORIZATION_HEADER.to_string(), auth);
    let mut in_req = InReq::new("/svc/M", vec![], Duration::from_secs(1))
        .with_metadata(GrpcMetadata { headers: in_headers });
    inbound.before_dispatch(&mut in_req).expect("inbound validate");

    assert_eq!(
        in_req
            .metadata
            .headers
            .get(EXTRACTED_BEARER_SUBJECT)
            .map(String::as_str),
        Some("alice"),
    );
}

/// @covers: round-trip with mismatched issuer fails closed.
#[test]
fn bearer_round_trip_rejects_mismatched_issuer_int_test() {
    let outbound = BearerOutboundInterceptor::from_config(outbound_cfg());
    let inbound  = BearerInboundInterceptor::from_config(BearerInboundConfig {
        expected_issuer: "different-issuer".into(),
        ..inbound_cfg()
    });

    let mut out_req = OutReq::new("/svc/M", vec![], Duration::from_secs(1));
    outbound.before_call(&mut out_req).expect("mint");
    let auth = out_req
        .metadata
        .headers
        .get(AUTHORIZATION_HEADER)
        .cloned()
        .unwrap();

    let mut in_headers = std::collections::HashMap::new();
    in_headers.insert(AUTHORIZATION_HEADER.to_string(), auth);
    let mut in_req = InReq::new("/svc/M", vec![], Duration::from_secs(1))
        .with_metadata(GrpcMetadata { headers: in_headers });
    match inbound.before_dispatch(&mut in_req) {
        Err(GrpcInboundError::Status(GrpcStatusCode::Unauthenticated, _)) => {}
        other => panic!("expected Unauthenticated, got {other:?}"),
    }
}
