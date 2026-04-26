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
    GrpcHealthCheck, GrpcInbound, GrpcInboundError, GrpcInboundResult, GrpcMessageStream,
    GrpcMetadata, GrpcRequest, GrpcResponse, TonicGrpcServer,
};

// ── Test handlers ─────────────────────────────────────────────────────────────

/// Handler whose `handle_stream` always returns exactly three fixed response frames.
struct ThreeFrameServerStreamHandler;

impl GrpcInbound for ThreeFrameServerStreamHandler {
    fn handle_unary(&self, _req: GrpcRequest) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            Ok(GrpcResponse { body: vec![], metadata: GrpcMetadata { headers: HashMap::new() } })
        })
    }

    fn handle_stream(
        &self,
        _method: String,
        _metadata: GrpcMetadata,
        _messages: GrpcMessageStream,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<(GrpcMessageStream, GrpcMetadata)>> {
        Box::pin(async move {
            let out: GrpcMessageStream = Box::pin(futures::stream::iter(vec![
                Ok(vec![1u8]),
                Ok(vec![2u8]),
                Ok(vec![3u8]),
            ]));
            Ok((out, GrpcMetadata::default()))
        })
    }

    fn health_check(&self) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move { Ok(GrpcHealthCheck::healthy()) })
    }
}

/// Handler whose `handle_stream` counts input frames and returns the count as a 1-byte response.
struct FrameCountHandler;

impl GrpcInbound for FrameCountHandler {
    fn handle_unary(&self, _req: GrpcRequest) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            Ok(GrpcResponse { body: vec![], metadata: GrpcMetadata::default() })
        })
    }

    fn handle_stream(
        &self,
        _method: String,
        _metadata: GrpcMetadata,
        messages: GrpcMessageStream,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<(GrpcMessageStream, GrpcMetadata)>> {
        Box::pin(async move {
            use futures::StreamExt;
            let count = messages.count().await;
            let out: GrpcMessageStream = Box::pin(futures::stream::iter(vec![
                Ok(vec![count as u8]),
            ]));
            Ok((out, GrpcMetadata::default()))
        })
    }

    fn health_check(&self) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move { Ok(GrpcHealthCheck::healthy()) })
    }
}

/// Handler whose `handle_stream` echoes every input frame back as a separate output frame.
struct EchoStreamHandler;

impl GrpcInbound for EchoStreamHandler {
    fn handle_unary(&self, req: GrpcRequest) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            Ok(GrpcResponse { body: req.body, metadata: GrpcMetadata::default() })
        })
    }

    fn handle_stream(
        &self,
        _method: String,
        _metadata: GrpcMetadata,
        messages: GrpcMessageStream,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<(GrpcMessageStream, GrpcMetadata)>> {
        Box::pin(async move {
            use futures::StreamExt;
            let items: Vec<GrpcInboundResult<Vec<u8>>> = messages.collect().await;
            let out: GrpcMessageStream = Box::pin(futures::stream::iter(items));
            Ok((out, GrpcMetadata::default()))
        })
    }

    fn health_check(&self) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move { Ok(GrpcHealthCheck::healthy()) })
    }
}

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

struct InvalidArgumentHandler;

impl GrpcInbound for InvalidArgumentHandler {
    fn handle_unary(&self, _: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move { Err(GrpcInboundError::InvalidArgument("bad field".into())) })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move { Ok(GrpcHealthCheck::healthy()) })
    }
}

struct DeadlineExceededHandler;

impl GrpcInbound for DeadlineExceededHandler {
    fn handle_unary(&self, _: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move { Err(GrpcInboundError::DeadlineExceeded("took too long".into())) })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move { Ok(GrpcHealthCheck::healthy()) })
    }
}

struct PermissionDeniedHandler;

impl GrpcInbound for PermissionDeniedHandler {
    fn handle_unary(&self, _: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move { Err(GrpcInboundError::PermissionDenied("not allowed".into())) })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move { Ok(GrpcHealthCheck::healthy()) })
    }
}

struct UnimplementedHandler;

impl GrpcInbound for UnimplementedHandler {
    fn handle_unary(&self, _: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move { Err(GrpcInboundError::Unimplemented("not built yet".into())) })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move { Ok(GrpcHealthCheck::healthy()) })
    }
}

/// Handler that returns custom metadata headers alongside an echo response.
struct MetadataHandler;

impl GrpcInbound for MetadataHandler {
    fn handle_unary(&self, req: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            let mut headers = HashMap::new();
            headers.insert("x-response-id".to_string(), "meta-42".to_string());
            Ok(GrpcResponse { body: req.body, metadata: GrpcMetadata { headers } })
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move { Ok(GrpcHealthCheck::healthy()) })
    }
}

/// Handler whose response stream yields one successful frame then an error.
struct MidStreamErrorHandler;

impl GrpcInbound for MidStreamErrorHandler {
    fn handle_unary(&self, _: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move { Ok(GrpcResponse { body: vec![], metadata: GrpcMetadata::default() }) })
    }

    fn handle_stream(
        &self,
        _method: String,
        _metadata: GrpcMetadata,
        _messages: GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcInboundResult<(GrpcMessageStream, GrpcMetadata)>> {
        Box::pin(async move {
            let out: GrpcMessageStream = Box::pin(futures::stream::iter(vec![
                Ok(vec![0u8]),
                Err(GrpcInboundError::Internal("mid-stream fail".into())),
            ]));
            Ok((out, GrpcMetadata::default()))
        })
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

/// Concatenate multiple payloads into a single body containing one gRPC frame per payload.
fn grpc_frames_body(payloads: &[&[u8]]) -> Bytes {
    let total = payloads.iter().map(|p| 5 + p.len()).sum();
    let mut buf = BytesMut::with_capacity(total);
    for p in payloads {
        buf.put_u8(0);
        buf.put_u32(p.len() as u32);
        buf.put_slice(p);
    }
    buf.freeze()
}

/// Parse all gRPC frames from a body, returning the raw payloads.
fn parse_grpc_frames(data: &Bytes) -> Vec<Bytes> {
    const HEADER: usize = 5;
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset + HEADER <= data.len() {
        let len = u32::from_be_bytes([
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
        ]) as usize;
        let start = offset + HEADER;
        let end   = start + len;
        if end > data.len() { break; }
        out.push(data.slice(start..end));
        offset = end;
    }
    out
}

/// Open an HTTP/2 prior-knowledge connection, POST a gRPC frame to `path`,
/// and return `(http-status, grpc-status-trailer, body-data-bytes)`.
async fn grpc_call(
    addr:    SocketAddr,
    path:    &str,
    payload: &[u8],
) -> (StatusCode, Option<String>, Bytes) {
    grpc_call_body(addr, path, grpc_frame(payload)).await
}

/// Like `grpc_call` but accepts a pre-built request body (may contain multiple frames).
async fn grpc_call_body(
    addr: SocketAddr,
    path: &str,
    body: Bytes,
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
        .body(Full::new(body))
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

/// Like `grpc_call` but also returns all trailer headers for metadata assertions.
async fn grpc_call_with_trailers(
    addr:    SocketAddr,
    path:    &str,
    payload: &[u8],
) -> (StatusCode, Option<String>, Bytes, HashMap<String, String>) {
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
    let trailers: HashMap<String, String> = collected
        .trailers()
        .map(|t| {
            t.iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|vs| (k.to_string(), vs.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let data = collected.to_bytes();
    (status, grpc_status, data, trailers)
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

// ── Streaming tests ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_server_streaming_returns_multiple_response_frames() {
    // ThreeFrameServerStreamHandler overrides handle_stream to produce 3 frames:
    // payloads [1], [2], [3].  The test verifies the wire response contains exactly
    // three 5-byte-prefixed frames.
    let (addr, _shutdown) = start_server(ThreeFrameServerStreamHandler).await;

    let (status, grpc_status, data) =
        grpc_call(addr, "/pkg.Svc/Stream", b"trigger").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(grpc_status.as_deref(), Some("0"));

    let frames = parse_grpc_frames(&data);
    assert_eq!(frames.len(), 3, "expected exactly 3 response frames, got {}", frames.len());
    assert_eq!(frames[0].as_ref(), &[1u8], "frame 0 payload mismatch");
    assert_eq!(frames[1].as_ref(), &[2u8], "frame 1 payload mismatch");
    assert_eq!(frames[2].as_ref(), &[3u8], "frame 2 payload mismatch");
}

#[tokio::test]
async fn test_client_streaming_sends_multiple_request_frames() {
    // FrameCountHandler overrides handle_stream to count input items and respond
    // with a single byte equal to the count.  Client sends 2 frames; expect count == 2.
    let (addr, _shutdown) = start_server(FrameCountHandler).await;

    let body = grpc_frames_body(&[b"frame-a", b"frame-b"]);
    let (status, grpc_status, data) =
        grpc_call_body(addr, "/pkg.Svc/ClientStream", body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(grpc_status.as_deref(), Some("0"));

    let frames = parse_grpc_frames(&data);
    assert_eq!(frames.len(), 1, "expected a single count frame");
    assert_eq!(frames[0].as_ref(), &[2u8], "handler should have seen 2 input frames");
}

#[tokio::test]
async fn test_bidi_streaming_echoes_all_messages() {
    // EchoStreamHandler overrides handle_stream to echo every input frame.
    // Client sends 3 frames with distinct payloads; response must contain all 3.
    let (addr, _shutdown) = start_server(EchoStreamHandler).await;

    let payloads: &[&[u8]] = &[b"alpha", b"beta", b"gamma"];
    let body = grpc_frames_body(payloads);
    let (status, grpc_status, data) =
        grpc_call_body(addr, "/pkg.Svc/Bidi", body).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(grpc_status.as_deref(), Some("0"));

    let frames = parse_grpc_frames(&data);
    assert_eq!(frames.len(), 3, "expected 3 echoed frames, got {}", frames.len());
    for (i, expected) in payloads.iter().enumerate() {
        assert_eq!(frames[i].as_ref(), *expected, "frame {i} payload mismatch");
    }
}

// ── Error variant coverage ────────────────────────────────────────────────────

#[tokio::test]
async fn test_server_returns_grpc_invalid_argument_status_for_bad_input() {
    let (addr, _shutdown) = start_server(InvalidArgumentHandler).await;
    let (status, grpc_status, _) = grpc_call(addr, "/pkg.Svc/Create", b"x").await;
    assert_eq!(status, StatusCode::OK);
    // tonic::Code::InvalidArgument == 3
    assert_eq!(grpc_status.as_deref(), Some("3"));
}

#[tokio::test]
async fn test_server_returns_grpc_deadline_exceeded_status_for_timeout_error() {
    let (addr, _shutdown) = start_server(DeadlineExceededHandler).await;
    let (status, grpc_status, _) = grpc_call(addr, "/pkg.Svc/Slow", b"x").await;
    assert_eq!(status, StatusCode::OK);
    // tonic::Code::DeadlineExceeded == 4
    assert_eq!(grpc_status.as_deref(), Some("4"));
}

#[tokio::test]
async fn test_server_returns_grpc_permission_denied_status_for_auth_error() {
    let (addr, _shutdown) = start_server(PermissionDeniedHandler).await;
    let (status, grpc_status, _) = grpc_call(addr, "/pkg.Svc/Admin", b"x").await;
    assert_eq!(status, StatusCode::OK);
    // tonic::Code::PermissionDenied == 7
    assert_eq!(grpc_status.as_deref(), Some("7"));
}

#[tokio::test]
async fn test_server_returns_grpc_unimplemented_status_for_unknown_method() {
    let (addr, _shutdown) = start_server(UnimplementedHandler).await;
    let (status, grpc_status, _) = grpc_call(addr, "/pkg.Svc/Unknown", b"x").await;
    assert_eq!(status, StatusCode::OK);
    // tonic::Code::Unimplemented == 12
    assert_eq!(grpc_status.as_deref(), Some("12"));
}

// ── Response metadata threading ───────────────────────────────────────────────

#[tokio::test]
async fn test_server_threads_response_metadata_into_trailers() {
    // MetadataHandler returns x-response-id: meta-42 in its GrpcResponse.metadata.
    // After the fix, grpc_stream_response threads those into HTTP/2 trailers.
    let (addr, _shutdown) = start_server(MetadataHandler).await;
    let (status, grpc_status, _, trailers) =
        grpc_call_with_trailers(addr, "/pkg.Svc/Meta", b"ping").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(grpc_status.as_deref(), Some("0"));
    assert_eq!(
        trailers.get("x-response-id").map(String::as_str),
        Some("meta-42"),
        "response metadata must be forwarded as HTTP/2 trailers; got trailers: {trailers:?}"
    );
}

// ── Mid-stream output error ───────────────────────────────────────────────────

#[tokio::test]
async fn test_server_returns_grpc_internal_status_for_mid_stream_output_error() {
    // MidStreamErrorHandler returns a stream: Ok([0]) then Err(Internal).
    // The server must stop collecting, discard partial output, and return grpc-status=13.
    let (addr, _shutdown) = start_server(MidStreamErrorHandler).await;
    let (status, grpc_status, _) = grpc_call(addr, "/pkg.Svc/Broken", b"x").await;
    assert_eq!(status, StatusCode::OK);
    // tonic::Code::Internal == 13
    assert_eq!(grpc_status.as_deref(), Some("13"));
}
