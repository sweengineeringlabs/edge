//! Registry-backed [`GrpcInbound`] dispatcher.
//!
//! Wraps an `Arc<HandlerRegistry<Vec<u8>, Vec<u8>>>` from `edge-domain`
//! and implements [`GrpcInbound`] by:
//!
//! 1. taking the gRPC method path off the inbound request,
//! 2. looking up the matching `Handler<Vec<u8>, Vec<u8>>` in the
//!    registry (typically a [`GrpcHandlerAdapter`] registered under the
//!    method path),
//! 3. forwarding the raw request bytes to `Handler::execute`,
//! 4. wrapping the response bytes in a [`GrpcResponse`].
//!
//! Method-not-found is reported as `tonic::Code::Unimplemented` —
//! aligning with how Google's reference servers respond when a service
//! does not advertise a method.

use std::sync::Arc;

use edge_domain::{HandlerError, HandlerRegistry};
use futures::future::BoxFuture;

use crate::api::handler_adapter::GrpcHandlerAdapter;
use crate::api::port::grpc_inbound::{
    GrpcHealthCheck, GrpcInbound, GrpcInboundError, GrpcInboundResult,
};
use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse};

/// Dispatcher that routes inbound gRPC calls through a byte-oriented
/// [`HandlerRegistry`] keyed by the gRPC method path.
///
/// Use this together with [`GrpcHandlerAdapter`] to register typed
/// handlers under their gRPC method paths and let the server dispatch
/// the right one for each inbound request.
pub struct HandlerRegistryDispatcher {
    registry: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>>,
}

impl HandlerRegistryDispatcher {
    /// Construct a dispatcher backed by `registry`.
    pub fn new(registry: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>>) -> Self {
        Self { registry }
    }

    /// Register a typed adapter under its `id()` (which by convention
    /// is the gRPC method path).
    pub fn register<Req, Resp>(&self, adapter: GrpcHandlerAdapter<Req, Resp>)
    where
        Req:  Send + 'static,
        Resp: Send + 'static,
    {
        self.registry.register(Arc::new(adapter));
    }

    /// Borrow the inner registry — callers can list ids, deregister,
    /// or share the registry with administrative tooling.
    pub fn registry(&self) -> &Arc<HandlerRegistry<Vec<u8>, Vec<u8>>> {
        &self.registry
    }
}

impl GrpcInbound for HandlerRegistryDispatcher {
    fn handle_unary(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        let registry = self.registry.clone();
        Box::pin(async move {
            let method = request.method.clone();
            let handler = match registry.get(&method) {
                Some(h) => h,
                None => {
                    return Err(GrpcInboundError::Unimplemented(format!(
                        "no handler registered for {method}"
                    )));
                }
            };
            match handler.execute(request.body).await {
                Ok(bytes) => Ok(GrpcResponse {
                    body:     bytes,
                    metadata: GrpcMetadata::default(),
                }),
                Err(e) => Err(map_handler_error(e)),
            }
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        let registry = self.registry.clone();
        Box::pin(async move {
            // Aggregate: dispatcher is healthy iff every registered handler is.
            let ids = registry.list_ids();
            for id in ids {
                if let Some(h) = registry.get(&id) {
                    if !h.health_check().await {
                        return Ok(GrpcHealthCheck::unhealthy(format!(
                            "handler {id} reported unhealthy"
                        )));
                    }
                }
            }
            Ok(GrpcHealthCheck::healthy())
        })
    }
}

/// Translate a [`HandlerError`] into the closest gRPC-shaped
/// [`GrpcInboundError`] without leaking internals to the wire.
fn map_handler_error(err: HandlerError) -> GrpcInboundError {
    match err {
        HandlerError::Unsupported(m)        => GrpcInboundError::Unimplemented(m),
        HandlerError::InvalidRequest(m)     => GrpcInboundError::InvalidArgument(m),
        HandlerError::ExecutionFailed(m)    => GrpcInboundError::Internal(m),
        HandlerError::Unhealthy             => GrpcInboundError::Unavailable("handler unhealthy".into()),
        HandlerError::FailedPrecondition(m) => GrpcInboundError::Status(crate::saf::GrpcStatusCode::FailedPrecondition, m),
        HandlerError::Other(m)              => GrpcInboundError::Internal(m),
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::time::Duration;

    use async_trait::async_trait;
    use edge_domain::{Handler, HandlerError, HandlerRegistry};

    use super::*;
    use crate::api::handler_adapter::GrpcHandlerAdapter;
    use crate::api::port::grpc_inbound::GrpcInbound;
    use crate::api::value_object::GrpcRequest;

    #[derive(Debug, PartialEq, Eq)]
    struct TestReq { value: u32 }

    #[derive(Debug, PartialEq, Eq)]
    struct TestResp { value: u32 }

    fn decode_test_req(bytes: &[u8]) -> Result<TestReq, GrpcInboundError> {
        if bytes.len() != 4 {
            return Err(GrpcInboundError::InvalidArgument(
                format!("expected 4 bytes, got {}", bytes.len())
            ));
        }
        Ok(TestReq { value: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) })
    }
    fn encode_test_resp(resp: &TestResp) -> Vec<u8> { resp.value.to_be_bytes().to_vec() }

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

    fn fresh_dispatcher() -> HandlerRegistryDispatcher {
        let registry: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>> = Arc::new(HandlerRegistry::new());
        HandlerRegistryDispatcher::new(registry)
    }

    /// @covers: HandlerRegistryDispatcher::new — empty registry has no handlers.
    #[tokio::test]
    async fn test_new_dispatcher_starts_with_no_registered_handlers() {
        let d = fresh_dispatcher();
        assert!(d.registry().is_empty());
    }

    /// @covers: HandlerRegistryDispatcher::register — adds an adapter under its id.
    #[tokio::test]
    async fn test_register_inserts_adapter_under_handler_id() {
        let d = fresh_dispatcher();
        d.register(GrpcHandlerAdapter::new(
            Arc::new(DoublingHandler),
            decode_test_req,
            encode_test_resp,
        ));
        assert_eq!(d.registry().len(), 1);
        assert!(d.registry().get("/pkg.Service/Double").is_some());
    }

    /// @covers: handle_unary — typed handler runs, response bytes returned.
    #[tokio::test]
    async fn test_handle_unary_runs_typed_handler_and_returns_encoded_response() {
        let d = fresh_dispatcher();
        d.register(GrpcHandlerAdapter::new(
            Arc::new(DoublingHandler),
            decode_test_req,
            encode_test_resp,
        ));
        let req_bytes = 21u32.to_be_bytes().to_vec();
        let req = GrpcRequest::new("/pkg.Service/Double", req_bytes, Duration::from_secs(1));
        let resp = d.handle_unary(req).await.expect("dispatch ok");
        let out = u32::from_be_bytes([resp.body[0], resp.body[1], resp.body[2], resp.body[3]]);
        assert_eq!(out, 42);
    }

    /// @covers: handle_unary — unknown method returns Unimplemented.
    #[tokio::test]
    async fn test_handle_unary_returns_unimplemented_when_method_not_registered() {
        let d = fresh_dispatcher();
        let req = GrpcRequest::new("/pkg.Service/NotThere", vec![], Duration::from_secs(1));
        let err = d.handle_unary(req).await.expect_err("unknown method must error");
        match err {
            GrpcInboundError::Unimplemented(msg) => {
                assert!(msg.contains("/pkg.Service/NotThere"), "{msg}");
            }
            other => panic!("expected Unimplemented, got {other:?}"),
        }
    }

    /// @covers: handle_unary — handler decode failure surfaces as InvalidArgument.
    #[tokio::test]
    async fn test_handle_unary_returns_invalid_argument_when_decode_fails() {
        let d = fresh_dispatcher();
        d.register(GrpcHandlerAdapter::new(
            Arc::new(DoublingHandler),
            decode_test_req,
            encode_test_resp,
        ));
        let bad = vec![1u8, 2, 3];
        let req = GrpcRequest::new("/pkg.Service/Double", bad, Duration::from_secs(1));
        let err = d.handle_unary(req).await.expect_err("must error");
        match err {
            GrpcInboundError::InvalidArgument(_) => {}
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    /// @covers: health_check — empty registry is healthy.
    #[tokio::test]
    async fn test_health_check_returns_healthy_for_empty_registry() {
        let d = fresh_dispatcher();
        let h = d.health_check().await.expect("health check ok");
        assert!(h.healthy);
    }

    /// @covers: health_check — unhealthy handler taints aggregate.
    #[tokio::test]
    async fn test_health_check_returns_unhealthy_when_any_handler_is_unhealthy() {
        struct SickHandler;
        #[async_trait]
        impl Handler<TestReq, TestResp> for SickHandler {
            fn id(&self) -> &str { "/pkg.Service/Sick" }
            fn pattern(&self) -> &str { "test" }
            async fn execute(&self, _: TestReq) -> Result<TestResp, HandlerError> {
                Err(HandlerError::Unhealthy)
            }
            async fn health_check(&self) -> bool { false }
            fn as_any(&self) -> &dyn Any { self }
        }
        let d = fresh_dispatcher();
        d.register(GrpcHandlerAdapter::new(
            Arc::new(DoublingHandler),
            decode_test_req,
            encode_test_resp,
        ));
        d.register(GrpcHandlerAdapter::new(
            Arc::new(SickHandler),
            decode_test_req,
            encode_test_resp,
        ));
        let h = d.health_check().await.expect("health check ok");
        assert!(!h.healthy);
        assert!(h.message.unwrap().contains("/pkg.Service/Sick"));
    }

    /// @covers: map_handler_error — Unsupported -> Unimplemented.
    #[test]
    fn test_map_handler_error_unsupported_maps_to_unimplemented() {
        let mapped = map_handler_error(HandlerError::Unsupported("nope".into()));
        assert!(matches!(mapped, GrpcInboundError::Unimplemented(_)));
    }

    /// @covers: map_handler_error — InvalidRequest -> InvalidArgument.
    #[test]
    fn test_map_handler_error_invalid_request_maps_to_invalid_argument() {
        let mapped = map_handler_error(HandlerError::InvalidRequest("bad".into()));
        assert!(matches!(mapped, GrpcInboundError::InvalidArgument(_)));
    }

    /// @covers: map_handler_error — ExecutionFailed -> Internal.
    #[test]
    fn test_map_handler_error_execution_failed_maps_to_internal() {
        let mapped = map_handler_error(HandlerError::ExecutionFailed("kaboom".into()));
        assert!(matches!(mapped, GrpcInboundError::Internal(_)));
    }

    /// @covers: map_handler_error — Unhealthy -> Unavailable.
    #[test]
    fn test_map_handler_error_unhealthy_maps_to_unavailable() {
        let mapped = map_handler_error(HandlerError::Unhealthy);
        assert!(matches!(mapped, GrpcInboundError::Unavailable(_)));
    }

    /// @covers: map_handler_error — Other -> Internal.
    #[test]
    fn test_map_handler_error_other_maps_to_internal() {
        let mapped = map_handler_error(HandlerError::Other("misc".into()));
        assert!(matches!(mapped, GrpcInboundError::Internal(_)));
    }

    /// @covers: map_handler_error — FailedPrecondition -> Status(FailedPrecondition).
    #[test]
    fn test_map_handler_error_failed_precondition_maps_to_status() {
        use crate::saf::GrpcStatusCode;
        let mapped = map_handler_error(HandlerError::FailedPrecondition("rerank not configured".into()));
        match mapped {
            GrpcInboundError::Status(code, msg) => {
                assert_eq!(code, GrpcStatusCode::FailedPrecondition);
                assert!(msg.contains("rerank not configured"));
            }
            other => panic!("expected Status(FailedPrecondition, _), got {other:?}"),
        }
    }
}
