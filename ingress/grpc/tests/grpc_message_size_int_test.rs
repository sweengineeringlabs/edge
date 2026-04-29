//! Issue #5 acceptance gate (3/4): an oversized inbound message must
//! return `ResourceExhausted` to the client and MUST NOT exhaust
//! server memory or invoke the handler.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use http::Request;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use swe_edge_ingress_grpc::{
    GrpcHealthCheck, GrpcInbound, GrpcInboundResult, GrpcMetadata, GrpcRequest, GrpcResponse,
    TonicGrpcServer,
};

struct GuardedHandler {
    hit: Arc<AtomicBool>,
}

impl GrpcInbound for GuardedHandler {
    fn handle_unary(
        &self,
        req: GrpcRequest,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        let hit = self.hit.clone();
        Box::pin(async move {
            hit.store(true, Ordering::SeqCst);
            Ok(GrpcResponse { body: req.body, metadata: GrpcMetadata::default() })
        })
    }

    fn health_check(
        &self,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async { Ok(GrpcHealthCheck::healthy()) })
    }
}

fn grpc_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

/// @covers: oversized inbound message returns `ResourceExhausted` and
/// MUST NOT invoke the handler.  Issue #5 acceptance gate (3/4).
#[tokio::test]
async fn server_returns_resource_exhausted_for_oversized_message_int_test() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let hit      = Arc::new(AtomicBool::new(false));

    // 1 KiB cap — sized to make the test fast while still exceeding the
    // handler envelope.  4 MiB is the production default.
    let max_bytes = 1024;
    let server   = TonicGrpcServer::new("127.0.0.1:0", Arc::new(GuardedHandler { hit: hit.clone() }))
        .with_max_message_size(max_bytes);

    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Send a payload that's clearly over the cap.
    let oversized = vec![0xAB_u8; max_bytes * 4];
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io     = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(io)
        .await
        .unwrap();
    tokio::spawn(conn);

    let req = Request::builder()
        .method("POST")
        .uri(format!("http://{addr}/pkg.Svc/Big"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(Full::new(grpc_frame(&oversized)))
        .unwrap();

    let resp = sender.send_request(req).await.unwrap();
    let collected = resp.into_body().collect().await.unwrap();
    let grpc_status = collected
        .trailers()
        .and_then(|t| t.get("grpc-status"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    // tonic::Code::ResourceExhausted = 8
    assert_eq!(
        grpc_status.as_deref(),
        Some("8"),
        "expected ResourceExhausted (8), got {grpc_status:?}",
    );
    assert!(
        !hit.load(Ordering::SeqCst),
        "handler MUST NOT execute on oversized message",
    );

    let _ = tx.send(());
}

/// @covers: messages within the cap pass through normally — companion
/// test to ensure the cap doesn't false-positive small payloads.
#[tokio::test]
async fn server_accepts_message_within_cap_int_test() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let hit      = Arc::new(AtomicBool::new(false));

    let max_bytes = 1024;
    let server   = TonicGrpcServer::new("127.0.0.1:0", Arc::new(GuardedHandler { hit: hit.clone() }))
        .with_max_message_size(max_bytes);

    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(20)).await;

    let payload = vec![0xCD_u8; 200];
    let stream  = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io      = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(io)
        .await
        .unwrap();
    tokio::spawn(conn);

    let req = Request::builder()
        .method("POST")
        .uri(format!("http://{addr}/pkg.Svc/Small"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(Full::new(grpc_frame(&payload)))
        .unwrap();

    let resp = sender.send_request(req).await.unwrap();
    let collected = resp.into_body().collect().await.unwrap();
    let grpc_status = collected
        .trailers()
        .and_then(|t| t.get("grpc-status"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    assert_eq!(grpc_status.as_deref(), Some("0"));
    assert!(hit.load(Ordering::SeqCst), "handler should execute for in-cap message");

    let _ = tx.send(());
}
