//! Phase 3 acceptance integration tests for the registry-backed
//! [`HandlerRegistryDispatcher`] + [`GrpcHandlerAdapter`] bridge.
//!
//! These hit the full wire path through `hyper::client::conn::http2`
//! to prove that:
//!
//! 1. A typed `Handler<Req, Resp>` registered under a gRPC method
//!    path is dispatched correctly when the client targets that path.
//! 2. Method-not-found returns `Unimplemented` (12) on the wire.
//! 3. The handler decode failure surfaces as `InvalidArgument` (3).

use std::any::Any;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use http::Request;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use edge_domain::{Handler, HandlerError, HandlerRegistry};
use swe_edge_ingress_grpc::{
    GrpcHandlerAdapter, GrpcInboundError, HandlerRegistryDispatcher, TonicGrpcServer,
};

#[derive(Debug, PartialEq, Eq)]
struct TripleReq { value: u32 }

#[derive(Debug, PartialEq, Eq)]
struct TripleResp { value: u32 }

fn decode_triple_req(bytes: &[u8]) -> Result<TripleReq, GrpcInboundError> {
    if bytes.len() != 4 {
        return Err(GrpcInboundError::InvalidArgument(format!(
            "expected 4 bytes, got {}",
            bytes.len()
        )));
    }
    Ok(TripleReq { value: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) })
}

fn encode_triple_resp(resp: &TripleResp) -> Vec<u8> { resp.value.to_be_bytes().to_vec() }

struct TripleHandler;

#[async_trait]
impl Handler<TripleReq, TripleResp> for TripleHandler {
    fn id(&self) -> &str { "/pkg.Math/Triple" }
    fn pattern(&self) -> &str { "Math" }
    async fn execute(&self, req: TripleReq) -> Result<TripleResp, HandlerError> {
        Ok(TripleResp { value: req.value.wrapping_mul(3) })
    }
    async fn health_check(&self) -> bool { true }
    fn as_any(&self) -> &dyn Any { self }
}

fn grpc_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

async fn start_server_with_dispatcher(
    dispatcher: HandlerRegistryDispatcher,
) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let server   = TonicGrpcServer::new("127.0.0.1:0", Arc::new(dispatcher))
        .allow_unauthenticated(true);
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    (addr, tx)
}

async fn grpc_call(
    addr:    SocketAddr,
    path:    &str,
    payload: &[u8],
) -> (Option<String>, Bytes) {
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(io)
        .await
        .unwrap();
    tokio::spawn(conn);

    let req = Request::builder()
        .method("POST")
        .uri(format!("http://{addr}{path}"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(Full::new(grpc_frame(payload)))
        .unwrap();

    let resp      = sender.send_request(req).await.unwrap();
    let collected = resp.into_body().collect().await.unwrap();
    let grpc_status = collected
        .trailers()
        .and_then(|t| t.get("grpc-status"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let data = collected.to_bytes();
    (grpc_status, data)
}

/// @covers: GrpcHandlerAdapter + HandlerRegistryDispatcher — typed handler
/// runs end-to-end and returns the right response over the gRPC wire.
#[tokio::test]
async fn test_typed_handler_dispatched_through_registry_returns_correct_response() {
    let registry: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>> = Arc::new(HandlerRegistry::new());
    let dispatcher = HandlerRegistryDispatcher::new(registry);
    dispatcher.register(GrpcHandlerAdapter::new(
        Arc::new(TripleHandler),
        decode_triple_req,
        encode_triple_resp,
    ));

    let (addr, _shutdown) = start_server_with_dispatcher(dispatcher).await;

    // 7 * 3 = 21 — verify the typed handler ran with the right input.
    let req_bytes = 7u32.to_be_bytes();
    let (grpc_status, data) = grpc_call(addr, "/pkg.Math/Triple", &req_bytes).await;
    assert_eq!(grpc_status.as_deref(), Some("0"), "expected grpc-status=0");

    // Parse out the 5-byte gRPC frame header and the 4 response bytes.
    assert!(data.len() >= 5 + 4, "wire body too small: {}", data.len());
    let payload = &data[5..5 + 4];
    let value = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
    assert_eq!(value, 21, "TripleHandler should triple 7 → 21, got {value}");
}

/// @covers: HandlerRegistryDispatcher — unknown method returns Unimplemented (12).
#[tokio::test]
async fn test_unknown_method_returns_grpc_unimplemented_on_wire() {
    let registry: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>> = Arc::new(HandlerRegistry::new());
    let dispatcher = HandlerRegistryDispatcher::new(registry);
    let (addr, _shutdown) = start_server_with_dispatcher(dispatcher).await;
    let (grpc_status, _) = grpc_call(addr, "/pkg.Math/NotARealMethod", b"anything").await;
    // tonic::Code::Unimplemented == 12
    assert_eq!(grpc_status.as_deref(), Some("12"));
}

/// @covers: GrpcHandlerAdapter — decode failure surfaces as InvalidArgument (3).
#[tokio::test]
async fn test_decode_failure_surfaces_as_grpc_invalid_argument() {
    let registry: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>> = Arc::new(HandlerRegistry::new());
    let dispatcher = HandlerRegistryDispatcher::new(registry);
    dispatcher.register(GrpcHandlerAdapter::new(
        Arc::new(TripleHandler),
        decode_triple_req,
        encode_triple_resp,
    ));
    let (addr, _shutdown) = start_server_with_dispatcher(dispatcher).await;

    // 3 bytes — TripleHandler expects 4.
    let bad_payload = vec![1u8, 2, 3];
    let (grpc_status, _) = grpc_call(addr, "/pkg.Math/Triple", &bad_payload).await;
    // tonic::Code::InvalidArgument == 3
    assert_eq!(grpc_status.as_deref(), Some("3"));
}
