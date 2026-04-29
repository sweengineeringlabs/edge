//! Bridge between [`edge_domain::Handler`] and the gRPC inbound port.
//!
//! `GrpcHandlerAdapter<Req, Resp>` wraps an `Arc<dyn Handler<Req, Resp>>`
//! together with a pair of `decode` / `encode` function pointers and
//! exposes the wrapped unit as a [`Handler<Vec<u8>, Vec<u8>>`].  This
//! lets a single byte-oriented [`HandlerRegistry<Vec<u8>, Vec<u8>>`]
//! (from `edge-domain`) hold dispatch entries for every gRPC method
//! served by the server, regardless of the underlying typed
//! request/response pair.
//!
//! The dispatcher (see [`crate::core::handler_dispatch`]) reads the
//! gRPC method path from the inbound request, looks up the matching
//! adapter in the registry by `Handler::id`, decodes the request bytes,
//! invokes the typed handler, and encodes the response bytes back onto
//! the wire.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use edge_domain::{Handler, HandlerError};

use crate::api::port::grpc_inbound::GrpcInboundError;

/// Function pointer that decodes a typed request from raw protobuf bytes.
///
/// Implementations should return [`GrpcInboundError::InvalidArgument`]
/// when the bytes cannot be parsed — that surfaces as
/// `tonic::Code::InvalidArgument` on the wire.
pub type DecodeFn<Req> = fn(&[u8]) -> Result<Req, GrpcInboundError>;

/// Function pointer that encodes a typed response to raw protobuf bytes.
///
/// Implementations are infallible by contract — if encoding can fail
/// for the concrete type, wrap it inline (e.g. `prost::Message::encode`
/// can fail only when the buffer is too small, which never happens for
/// `Vec<u8>`).
pub type EncodeFn<Resp> = fn(&Resp) -> Vec<u8>;

/// Adapter that exposes a typed [`Handler<Req, Resp>`] as a
/// [`Handler<Vec<u8>, Vec<u8>>`] for registration in a single
/// byte-oriented [`HandlerRegistry`](edge_domain::HandlerRegistry).
///
/// `id` is forwarded verbatim from the inner handler — by convention
/// it is the fully-qualified gRPC method path
/// (e.g. `"/pkg.Service/Method"`).
pub struct GrpcHandlerAdapter<Req, Resp>
where
    Req:  Send + 'static,
    Resp: Send + 'static,
{
    inner:  Arc<dyn Handler<Req, Resp>>,
    decode: DecodeFn<Req>,
    encode: EncodeFn<Resp>,
}

impl<Req, Resp> GrpcHandlerAdapter<Req, Resp>
where
    Req:  Send + 'static,
    Resp: Send + 'static,
{
    /// Construct a new adapter from an inner typed handler and a
    /// decode/encode pair.
    pub fn new(
        inner:  Arc<dyn Handler<Req, Resp>>,
        decode: DecodeFn<Req>,
        encode: EncodeFn<Resp>,
    ) -> Self {
        Self { inner, decode, encode }
    }

    /// Borrow the inner typed handler — useful for administrative
    /// tooling that wants to interrogate the underlying domain unit.
    pub fn inner(&self) -> &Arc<dyn Handler<Req, Resp>> { &self.inner }
}

#[async_trait]
impl<Req, Resp> Handler<Vec<u8>, Vec<u8>> for GrpcHandlerAdapter<Req, Resp>
where
    Req:  Send + 'static,
    Resp: Send + 'static,
{
    fn id(&self) -> &str { self.inner.id() }

    fn pattern(&self) -> &str { self.inner.pattern() }

    async fn execute(&self, req: Vec<u8>) -> Result<Vec<u8>, HandlerError> {
        let typed = (self.decode)(&req).map_err(|e| match e {
            GrpcInboundError::InvalidArgument(msg) => HandlerError::InvalidRequest(msg),
            GrpcInboundError::Status(_, msg)       => HandlerError::InvalidRequest(msg),
            other                                  => HandlerError::InvalidRequest(other.to_string()),
        })?;
        let resp = self.inner.execute(typed).await?;
        Ok((self.encode)(&resp))
    }

    async fn health_check(&self) -> bool { self.inner.health_check().await }

    fn as_any(&self) -> &dyn Any { self }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct TestReq { value: u32 }

    #[derive(Debug, PartialEq, Eq)]
    struct TestResp { value: u32 }

    /// 4-byte big-endian decode — fails on anything else.
    fn decode_test_req(bytes: &[u8]) -> Result<TestReq, GrpcInboundError> {
        if bytes.len() != 4 {
            return Err(GrpcInboundError::InvalidArgument(
                format!("expected 4 bytes, got {}", bytes.len())
            ));
        }
        let value = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        Ok(TestReq { value })
    }

    fn encode_test_resp(resp: &TestResp) -> Vec<u8> {
        resp.value.to_be_bytes().to_vec()
    }

    struct DoublingHandler;

    #[async_trait]
    impl Handler<TestReq, TestResp> for DoublingHandler {
        fn id(&self) -> &str { "/pkg.Service/Double" }
        fn pattern(&self) -> &str { "test" }
        async fn execute(&self, req: TestReq) -> Result<TestResp, HandlerError> {
            Ok(TestResp { value: req.value.wrapping_mul(2) })
        }
        async fn health_check(&self) -> bool { true }
        fn as_any(&self) -> &dyn Any { self }
    }

    struct UnhealthyHandler;

    #[async_trait]
    impl Handler<TestReq, TestResp> for UnhealthyHandler {
        fn id(&self) -> &str { "/pkg.Service/Sick" }
        fn pattern(&self) -> &str { "test" }
        async fn execute(&self, _: TestReq) -> Result<TestResp, HandlerError> {
            Err(HandlerError::Unhealthy)
        }
        async fn health_check(&self) -> bool { false }
        fn as_any(&self) -> &dyn Any { self }
    }

    /// @covers: GrpcHandlerAdapter::new — id and pattern are forwarded verbatim.
    #[tokio::test]
    async fn test_new_forwards_id_and_pattern_from_inner_handler() {
        let adapter: GrpcHandlerAdapter<TestReq, TestResp> = GrpcHandlerAdapter::new(
            Arc::new(DoublingHandler),
            decode_test_req,
            encode_test_resp,
        );
        assert_eq!(adapter.id(),      "/pkg.Service/Double");
        assert_eq!(adapter.pattern(), "test");
    }

    /// @covers: GrpcHandlerAdapter::execute — decodes, runs inner, encodes.
    #[tokio::test]
    async fn test_execute_decodes_invokes_inner_and_encodes_response() {
        let adapter: GrpcHandlerAdapter<TestReq, TestResp> = GrpcHandlerAdapter::new(
            Arc::new(DoublingHandler),
            decode_test_req,
            encode_test_resp,
        );
        let req_bytes = 21u32.to_be_bytes().to_vec();
        let resp = (&adapter as &dyn Handler<Vec<u8>, Vec<u8>>)
            .execute(req_bytes).await.expect("execute");
        let value = u32::from_be_bytes([resp[0], resp[1], resp[2], resp[3]]);
        assert_eq!(value, 42, "DoublingHandler should multiply input by 2");
    }

    /// @covers: GrpcHandlerAdapter::execute — decode failure surfaces as InvalidRequest.
    #[tokio::test]
    async fn test_execute_returns_invalid_request_when_decode_fails() {
        let adapter: GrpcHandlerAdapter<TestReq, TestResp> = GrpcHandlerAdapter::new(
            Arc::new(DoublingHandler),
            decode_test_req,
            encode_test_resp,
        );
        let bad_bytes = vec![1u8, 2, 3]; // wrong length
        let err = (&adapter as &dyn Handler<Vec<u8>, Vec<u8>>)
            .execute(bad_bytes).await.expect_err("decode failure must error");
        match err {
            HandlerError::InvalidRequest(msg) => {
                assert!(msg.contains("4 bytes"), "msg should describe decode failure: {msg}");
            }
            other => panic!("expected InvalidRequest, got {other:?}"),
        }
    }

    /// @covers: GrpcHandlerAdapter::execute — inner handler error propagates.
    #[tokio::test]
    async fn test_execute_propagates_inner_handler_error() {
        let adapter: GrpcHandlerAdapter<TestReq, TestResp> = GrpcHandlerAdapter::new(
            Arc::new(UnhealthyHandler),
            decode_test_req,
            encode_test_resp,
        );
        let req_bytes = 1u32.to_be_bytes().to_vec();
        let err = (&adapter as &dyn Handler<Vec<u8>, Vec<u8>>)
            .execute(req_bytes).await.expect_err("unhealthy must error");
        assert!(matches!(err, HandlerError::Unhealthy), "got: {err:?}");
    }

    /// @covers: GrpcHandlerAdapter::health_check — forwarded from inner.
    #[tokio::test]
    async fn test_health_check_forwards_to_inner_handler() {
        let healthy: GrpcHandlerAdapter<TestReq, TestResp> = GrpcHandlerAdapter::new(
            Arc::new(DoublingHandler),
            decode_test_req,
            encode_test_resp,
        );
        let unhealthy: GrpcHandlerAdapter<TestReq, TestResp> = GrpcHandlerAdapter::new(
            Arc::new(UnhealthyHandler),
            decode_test_req,
            encode_test_resp,
        );
        assert!((&healthy as &dyn Handler<Vec<u8>, Vec<u8>>).health_check().await);
        assert!(!(&unhealthy as &dyn Handler<Vec<u8>, Vec<u8>>).health_check().await);
    }

    /// @covers: GrpcHandlerAdapter::inner — exposes the wrapped Arc<Handler>.
    #[tokio::test]
    async fn test_inner_returns_wrapped_handler() {
        let adapter: GrpcHandlerAdapter<TestReq, TestResp> = GrpcHandlerAdapter::new(
            Arc::new(DoublingHandler),
            decode_test_req,
            encode_test_resp,
        );
        assert_eq!(adapter.inner().id(), "/pkg.Service/Double");
    }
}
