//! W3C Trace Context propagation for outbound gRPC calls.
//!
//! Implements the [W3C Trace Context](https://www.w3.org/TR/trace-context/)
//! `traceparent` header.  When a request already carries a `traceparent`,
//! the interceptor preserves it.  Otherwise it can inject a static one.
//!
//! Propagation-only — does not start spans or sample.

use crate::api::interceptor::GrpcOutboundInterceptor;
use crate::api::port::GrpcOutboundError;
use crate::api::value_object::{GrpcRequest, GrpcResponse};

pub(crate) const TRACEPARENT: &str = "traceparent";
pub(crate) const TRACESTATE: &str = "tracestate";

/// Source of the trace context the interceptor will inject.
#[derive(Clone)]
pub enum TraceContextSource {
    /// No automatic injection.
    PassThrough,
    /// Inject a static `traceparent` (and optional `tracestate`).
    Static {
        /// Verbatim `traceparent` header value.
        traceparent: String,
        /// Optional `tracestate` header value.
        tracestate:  Option<String>,
    },
}

/// W3C Trace Context propagation interceptor for outbound gRPC.
#[derive(Clone)]
pub struct TraceContextInterceptor {
    source: TraceContextSource,
}

impl TraceContextInterceptor {
    /// Construct a passthrough interceptor — never injects.
    pub fn pass_through() -> Self {
        Self { source: TraceContextSource::PassThrough }
    }

    /// Construct an interceptor that injects a fixed `traceparent`
    /// when none is present.
    pub fn with_static(traceparent: impl Into<String>, tracestate: Option<String>) -> Self {
        Self {
            source: TraceContextSource::Static {
                traceparent: traceparent.into(),
                tracestate,
            },
        }
    }
}

impl GrpcOutboundInterceptor for TraceContextInterceptor {
    fn before_call(&self, req: &mut GrpcRequest) -> Result<(), GrpcOutboundError> {
        if req.metadata.headers.contains_key(TRACEPARENT) {
            return Ok(());
        }
        match &self.source {
            TraceContextSource::PassThrough => Ok(()),
            TraceContextSource::Static { traceparent, tracestate } => {
                req.metadata.headers.insert(TRACEPARENT.into(), traceparent.clone());
                if let Some(state) = tracestate {
                    req.metadata.headers.insert(TRACESTATE.into(), state.clone());
                }
                Ok(())
            }
        }
    }

    fn after_call(&self, _resp: &mut GrpcResponse) -> Result<(), GrpcOutboundError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse};

    use super::*;

    fn req() -> GrpcRequest {
        GrpcRequest::new("svc/M", vec![], Duration::from_secs(1))
    }

    /// @covers: pass_through — never injects.
    #[test]
    fn test_pass_through_does_not_inject_traceparent_when_absent() {
        let interceptor = TraceContextInterceptor::pass_through();
        let mut r = req();
        interceptor.before_call(&mut r).expect("before_call");
        assert!(r.metadata.headers.get(TRACEPARENT).is_none());
    }

    /// @covers: with_static — injects when absent.
    #[test]
    fn test_with_static_injects_traceparent_when_absent() {
        let tp = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let interceptor = TraceContextInterceptor::with_static(tp, None);
        let mut r = req();
        interceptor.before_call(&mut r).expect("before_call");
        assert_eq!(r.metadata.headers.get(TRACEPARENT).map(String::as_str), Some(tp));
    }

    /// @covers: with_static — injects tracestate too.
    #[test]
    fn test_with_static_injects_tracestate_when_configured() {
        let tp = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let interceptor = TraceContextInterceptor::with_static(tp, Some("v=1".to_string()));
        let mut r = req();
        interceptor.before_call(&mut r).expect("before_call");
        assert_eq!(r.metadata.headers.get(TRACESTATE).map(String::as_str), Some("v=1"));
    }

    /// @covers: upstream traceparent preserved.
    #[test]
    fn test_upstream_traceparent_is_preserved_and_not_overwritten() {
        let upstream = "00-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-bbbbbbbbbbbbbbbb-01";
        let injected = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let interceptor = TraceContextInterceptor::with_static(injected, None);
        let mut r = req();
        r.metadata.headers.insert(TRACEPARENT.into(), upstream.into());
        interceptor.before_call(&mut r).expect("before_call");
        assert_eq!(r.metadata.headers.get(TRACEPARENT).map(String::as_str), Some(upstream));
    }

    /// @covers: after_call — no-op.
    #[test]
    fn test_after_call_does_not_modify_response() {
        let interceptor = TraceContextInterceptor::pass_through();
        let mut resp = GrpcResponse {
            body:     vec![],
            metadata: GrpcMetadata::default(),
        };
        interceptor.after_call(&mut resp).expect("after_call");
        assert!(resp.metadata.headers.is_empty());
    }
}
