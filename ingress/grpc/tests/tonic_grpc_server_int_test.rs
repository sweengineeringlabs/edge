//! Integration tests for TonicGrpcServer — real HTTP/2 gRPC over TCP.
//!
//! Each test binds a port-0 listener, wires a [`GrpcInbound`] handler, and
//! exercises the full wire path through `hyper::client::conn::http2`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::{BufMut, Bytes, BytesMut};
use futures::future::BoxFuture;
use http::{Request, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use swe_edge_ingress_grpc::{
    GrpcHealthCheck, GrpcInbound, GrpcInboundError, GrpcInboundResult, GrpcMetadata, GrpcRequest,
    GrpcResponse, TonicGrpcServer,
};

// ── Test handlers ─────────────────────────────────────────────────────────────

struct EchoHandler;

impl GrpcInbound for EchoHandler {
    fn handle_unary(&self, req: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            Ok(GrpcResponse {
                body:     req.body,
                metadata: GrpcMetadata { headers: HashMap::new() },
            })
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move { Ok(GrpcHealthCheck::healthy()) })
    }
}

struct NotFoundHandler;

impl GrpcInbound for NotFoundHandler {
    fn handle_unary(&self, _: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move { Err(GrpcInboundError::NotFound("no such method".into())) })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move { Ok(GrpcHealthCheck::healthy()) })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn start_server<H: GrpcInbound + 'static>(handler: H) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let server   = TonicGrpcServer::new("127.0.0.1:0", Arc::new(handler));
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    (addr, tx)
}

/// Encode a raw payload as a gRPC length-prefix frame.
fn grpc_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0); // not compressed
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

/// Open an HTTP/2 prior-knowledge connection, POST a gRPC frame to `path`,
/// and return `(http-status, grpc-status-trailer, body-data-bytes)`.
async fn grpc_call(
    addr:    SocketAddr,
    path:    &str,
    payload: &[u8],
) -> (StatusCode, Option<String>, Bytes) {
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
    let status    = resp.status();
    let collected = resp.into_body().collect().await.unwrap();
    let grpc_status = collected
        .trailers()
        .and_then(|t| t.get("grpc-status"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let data = collected.to_bytes();

    (status, grpc_status, data)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_server_returns_200_with_grpc_content_type_for_successful_request() {
    let (addr, _shutdown) = start_server(EchoHandler).await;

    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(io).await.unwrap();
    tokio::spawn(conn);

    let req = Request::builder()
        .method("POST")
        .uri(format!("http://{addr}/pkg.Svc/Method"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(Full::new(grpc_frame(b"hello")))
        .unwrap();

    let resp = sender.send_request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/grpc"
    );
}

#[tokio::test]
async fn test_server_echoes_payload_in_grpc_data_frame_with_status_0() {
    let (addr, _shutdown) = start_server(EchoHandler).await;

    let payload = b"echo-me";
    let (status, grpc_status, data) = grpc_call(addr, "/pkg.Svc/Echo", payload).await;

    assert_eq!(status, StatusCode::OK);
    // grpc-status=0 means OK
    assert_eq!(grpc_status.as_deref(), Some("0"));
    // Response body: 5-byte frame prefix + echoed payload.
    assert!(data.len() >= 5 + payload.len());
    assert_eq!(data[0], 0, "compressed flag must be 0");
    let frame_len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
    assert_eq!(frame_len, payload.len());
    assert_eq!(&data[5..5 + payload.len()], payload);
}

#[tokio::test]
async fn test_server_returns_grpc_not_found_status_for_handler_error() {
    let (addr, _shutdown) = start_server(NotFoundHandler).await;

    let (status, grpc_status, _) = grpc_call(addr, "/pkg.Svc/Missing", b"x").await;

    assert_eq!(status, StatusCode::OK);
    // tonic::Code::NotFound == 5
    assert_eq!(grpc_status.as_deref(), Some("5"));
}

#[tokio::test]
async fn test_server_routes_any_path_to_handler() {
    let (addr, _shutdown) = start_server(EchoHandler).await;

    for path in ["/a", "/b/c/d", "/very/long/nested/path"] {
        let (status, grpc_status, _) = grpc_call(addr, path, b"x").await;
        assert_eq!(status, StatusCode::OK, "path={path}");
        assert_eq!(grpc_status.as_deref(), Some("0"), "path={path}");
    }
}

#[tokio::test]
async fn test_server_enforces_message_size_limit_with_resource_exhausted() {
    let listener  = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr      = listener.local_addr().unwrap();
    let server    = TonicGrpcServer::new("127.0.0.1:0", Arc::new(EchoHandler))
        .with_max_message_size(16);
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server.serve_with_listener(listener, async move { let _ = rx.await; }).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // 17 payload bytes + 5-byte frame = 22 total > 16-byte limit.
    let oversized = vec![0u8; 17];
    let (status, grpc_status, _) = grpc_call(addr, "/pkg.Svc/Big", &oversized).await;

    assert_eq!(status, StatusCode::OK);
    // tonic::Code::ResourceExhausted == 8
    assert_eq!(grpc_status.as_deref(), Some("8"));
    drop(tx);
}

#[tokio::test]
async fn test_server_graceful_shutdown_refuses_new_connections() {
    let (addr, shutdown_tx) = start_server(EchoHandler).await;

    // Verify working before shutdown.
    let (status, _, _) = grpc_call(addr, "/pkg.Svc/Pre", b"x").await;
    assert_eq!(status, StatusCode::OK);

    // Signal shutdown and let the listener close.
    drop(shutdown_tx);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // New TCP connections must be refused.
    let result = tokio::net::TcpStream::connect(addr).await;
    assert!(result.is_err(), "expected connection refused after shutdown");
}
