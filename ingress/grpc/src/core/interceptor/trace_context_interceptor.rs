//! W3C Trace Context extraction for inbound gRPC requests.
//!
//! Inverse of the egress interceptor: extracts an existing
//! `traceparent` (and optional `tracestate`), validates v00 wire
//! shape, and republishes the values under internal keys
//! `x-edge-extracted-traceparent` / `x-edge-extracted-tracestate`.
//!
//! Original headers are preserved verbatim — this interceptor only
//! **adds** keys.

use crate::api::interceptor::GrpcInboundInterceptor;
use crate::api::port::grpc_inbound::GrpcInboundError;
use crate::api::value_object::{GrpcRequest, GrpcResponse};

/// Header name (HTTP/2 lower-case) for the W3C `traceparent`.
pub const TRACEPARENT: &str = "traceparent";

/// Header name for the W3C `tracestate`.
pub const TRACESTATE: &str = "tracestate";

/// Internal metadata key under which the validated `traceparent` is
/// republished for downstream handlers.
pub const EXTRACTED_TRACEPARENT: &str = "x-edge-extracted-traceparent";

/// Internal metadata key under which the validated `tracestate` is
/// republished for downstream handlers.
pub const EXTRACTED_TRACESTATE: &str = "x-edge-extracted-tracestate";

/// W3C Trace Context inbound extractor.
#[derive(Clone, Default)]
pub struct TraceContextInterceptor {}

impl TraceContextInterceptor {
    /// Construct a default extractor.
    pub fn new() -> Self { Self::default() }
}

fn is_valid_traceparent(value: &str) -> bool {
    if value.len() != 55 {
        return false;
    }
    let bytes = value.as_bytes();
    if &bytes[0..2] != b"00" || bytes[2] != b'-' {
        return false;
    }
    if bytes[35] != b'-' || bytes[52] != b'-' {
        return false;
    }
    let hex = |slice: &[u8]| -> bool {
        slice.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    };
    hex(&bytes[3..35]) && hex(&bytes[36..52]) && hex(&bytes[53..55])
}

impl GrpcInboundInterceptor for TraceContextInterceptor {
    fn before_dispatch(&self, req: &mut GrpcRequest) -> Result<(), GrpcInboundError> {
        if let Some(tp) = req.metadata.headers.get(TRACEPARENT).cloned() {
            if is_valid_traceparent(&tp) {
                req.metadata
                    .headers
                    .insert(EXTRACTED_TRACEPARENT.to_string(), tp);
                if let Some(ts) = req.metadata.headers.get(TRACESTATE).cloned() {
                    req.metadata
                        .headers
                        .insert(EXTRACTED_TRACESTATE.to_string(), ts);
                }
            } else {
                tracing::warn!(value = %tp, "dropping malformed inbound traceparent");
            }
        }
        Ok(())
    }

    fn after_dispatch(&self, _resp: &mut GrpcResponse) -> Result<(), GrpcInboundError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn req() -> GrpcRequest {
        GrpcRequest::new("svc/M", vec![], Duration::from_secs(1))
    }

    /// @covers: extracts well-formed traceparent.
    #[test]
    fn test_extracts_well_formed_traceparent_into_internal_key() {
        let tp = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let interceptor = TraceContextInterceptor::new();
        let mut r = req();
        r.metadata.headers.insert(TRACEPARENT.into(), tp.into());
        interceptor.before_dispatch(&mut r).expect("before_dispatch");
        assert_eq!(
            r.metadata.headers.get(EXTRACTED_TRACEPARENT).map(String::as_str),
            Some(tp),
        );
    }

    /// @covers: extracts tracestate alongside.
    #[test]
    fn test_extracts_tracestate_when_traceparent_is_valid() {
        let tp = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let ts = "vendor=value";
        let interceptor = TraceContextInterceptor::new();
        let mut r = req();
        r.metadata.headers.insert(TRACEPARENT.into(), tp.into());
        r.metadata.headers.insert(TRACESTATE.into(), ts.into());
        interceptor.before_dispatch(&mut r).expect("before_dispatch");
        assert_eq!(
            r.metadata.headers.get(EXTRACTED_TRACESTATE).map(String::as_str),
            Some(ts),
        );
    }

    /// @covers: malformed traceparent is dropped silently.
    #[test]
    fn test_drops_malformed_traceparent_and_continues() {
        let interceptor = TraceContextInterceptor::new();
        let mut r = req();
        r.metadata.headers.insert(TRACEPARENT.into(), "garbage".into());
        interceptor.before_dispatch(&mut r).expect("before_dispatch");
        assert!(r.metadata.headers.get(EXTRACTED_TRACEPARENT).is_none());
    }

    /// @covers: absent traceparent is no-op.
    #[test]
    fn test_absent_traceparent_is_a_noop() {
        let interceptor = TraceContextInterceptor::new();
        let mut r = req();
        interceptor.before_dispatch(&mut r).expect("before_dispatch");
        assert!(r.metadata.headers.is_empty());
    }

    /// @covers: is_valid_traceparent — strict v00 shape.
    #[test]
    fn test_is_valid_traceparent_accepts_canonical_shape_only() {
        assert!(is_valid_traceparent(
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
        ));
        assert!(!is_valid_traceparent("00-tooshort"));
        assert!(!is_valid_traceparent(
            "01-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
        ));
        assert!(!is_valid_traceparent(
            "00-0AF7651916CD43DD8448EB211C80319C-b7ad6b7169203331-01"
        ));
    }
}
