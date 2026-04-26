//! Integration tests for `TonicGrpcClient` / `GrpcOutbound`.
//!
//! Tests spin up a minimal in-process HTTP/2 echo server (no external process).

use std::convert::Infallible;
use std::net::SocketAddr;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::stream;
use http_body_util::{BodyExt as _, Full};

use swe_edge_egress_grpc::{
    GrpcMessageStream, GrpcMetadata, GrpcOutbound, GrpcOutboundError, GrpcRequest, GrpcResponse,
    GrpcStatusCode, TonicGrpcClient,
};

// ── gRPC frame helpers (duplicated here to keep test self-contained) ─────────

fn encode_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0x00);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

fn decode_frames(mut data: Bytes) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while data.len() >= 5 {
        let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
        data.advance(5);
        if data.len() < len {
            break;
        }
        out.push(data[..len].to_vec());
        data.advance(len);
    }
    out
}

// ── test server helpers ───────────────────────────────────────────────────────

/// Bind to an OS-assigned port and return the address.
async fn bind_listener() -> tokio::net::TcpListener {
    tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener")
}

/// Spawn an echo gRPC server that reflects every request frame back as a response frame.
///
/// Returns the bound `SocketAddr`.
async fn spawn_echo_server(listener: tokio::net::TcpListener) -> SocketAddr {
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };

            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let _ = hyper::server::conn::http2::Builder::new(
                    hyper_util::rt::TokioExecutor::new(),
                )
                .serve_connection(
                    io,
                    hyper::service::service_fn(|req: http::Request<hyper::body::Incoming>| async {
                        // Collect body bytes.
                        let collected = req.into_body().collect().await.unwrap();
                        let body_bytes = collected.to_bytes();

                        // Decode frames and re-encode the payloads as response frames.
                        let frames = decode_frames(body_bytes);
                        let mut resp_buf = BytesMut::new();
                        for f in frames {
                            resp_buf.put(encode_frame(&f));
                        }

                        let resp = http::Response::builder()
                            .status(200)
                            .header(http::header::CONTENT_TYPE, "application/grpc")
                            .header("grpc-status", "0")
                            .body(Full::new(resp_buf.freeze()))
                            .unwrap();

                        Ok::<_, Infallible>(resp)
                    }),
                )
                .await;
            });
        }
    });

    addr
}

/// Spawn a server that always responds with grpc-status=13 (Internal).
async fn spawn_error_server(listener: tokio::net::TcpListener) -> SocketAddr {
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let _ = hyper::server::conn::http2::Builder::new(
                    hyper_util::rt::TokioExecutor::new(),
                )
                .serve_connection(
                    io,
                    hyper::service::service_fn(|_req: http::Request<hyper::body::Incoming>| async {
                        let resp = http::Response::builder()
                            .status(200)
                            .header(http::header::CONTENT_TYPE, "application/grpc")
                            .header("grpc-status", "13")
                            .header("grpc-message", "server-side error")
                            .body(Full::new(Bytes::new()))
                            .unwrap();
                        Ok::<_, Infallible>(resp)
                    }),
                )
                .await;
            });
        }
    });

    addr
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// @covers: TonicGrpcClient::call_unary — happy path echo.
#[tokio::test]
async fn test_call_unary_sends_request_and_receives_response() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let req = GrpcRequest {
        method: "echo/Echo".into(),
        body: b"hello".to_vec(),
        metadata: GrpcMetadata::default(),
    };

    let resp = client.call_unary(req).await.expect("call_unary should succeed");
    assert_eq!(resp.body, b"hello");
}

/// @covers: TonicGrpcClient::call_unary — non-zero grpc-status maps to Err.
#[tokio::test]
async fn test_call_unary_propagates_grpc_error_status() {
    let listener = bind_listener().await;
    let addr = spawn_error_server(listener).await;

    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let req = GrpcRequest {
        method: "echo/Echo".into(),
        body: b"ping".to_vec(),
        metadata: GrpcMetadata::default(),
    };

    let result = client.call_unary(req).await;
    assert!(
        result.is_err(),
        "expected Err for grpc-status 13, got Ok"
    );
    match result.unwrap_err() {
        GrpcOutboundError::Internal(msg) => {
            assert!(msg.contains("grpc-status 13"), "error message was: {msg}");
        }
        other => panic!("expected Internal error, got {other:?}"),
    }
}

/// @covers: TonicGrpcClient::call_stream — multiple frames echoed back.
#[tokio::test]
async fn test_call_stream_sends_multiple_frames_and_receives_stream() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let messages: GrpcMessageStream = Box::pin(stream::iter(vec![
        Ok(b"frame1".to_vec()),
        Ok(b"frame2".to_vec()),
        Ok(b"frame3".to_vec()),
    ]));

    let mut resp_stream = client
        .call_stream("echo/Echo".into(), GrpcMetadata::default(), messages)
        .await
        .expect("call_stream should succeed");

    let mut items: Vec<Vec<u8>> = Vec::new();
    while let Some(item) = {
        use futures::StreamExt as _;
        resp_stream.next().await
    } {
        items.push(item.expect("stream item should be Ok"));
    }

    assert_eq!(items.len(), 3, "expected 3 frames, got {}", items.len());
    assert_eq!(items[0], b"frame1");
    assert_eq!(items[1], b"frame2");
    assert_eq!(items[2], b"frame3");
}

/// @covers: TonicGrpcClient::health_check — succeeds when server is listening.
#[tokio::test]
async fn test_health_check_succeeds_when_server_is_listening() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    let client = TonicGrpcClient::new(format!("http://{addr}"));
    client.health_check().await.expect("health_check should succeed when port is open");
}

/// @covers: TonicGrpcClient::health_check — fails when nothing is listening.
#[tokio::test]
async fn test_health_check_fails_when_no_server_is_listening() {
    // Bind to get an OS-assigned port, then drop the listener so nothing is listening on it.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    drop(listener);

    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let result = client.health_check().await;
    assert!(
        result.is_err(),
        "expected Err when nothing is listening, got Ok"
    );
    match result.unwrap_err() {
        GrpcOutboundError::Unavailable(msg) => {
            assert!(
                msg.contains(&addr.port().to_string()),
                "error should mention port, got: {msg}"
            );
        }
        other => panic!("expected Unavailable error, got {other:?}"),
    }
}

// ── value-object regression tests (kept from original file) ──────────────────

/// @covers: GrpcRequest — struct construction.
#[test]
fn test_grpc_request_holds_method_and_body() {
    let req = GrpcRequest {
        method: "svc/Method".into(),
        body: vec![1, 2, 3],
        metadata: GrpcMetadata::default(),
    };
    assert_eq!(req.method, "svc/Method");
    assert_eq!(req.body, vec![1, 2, 3]);
}

/// @covers: GrpcMetadata::default — starts with empty headers.
#[test]
fn test_grpc_metadata_default_has_empty_headers() {
    let m = GrpcMetadata::default();
    assert!(m.headers.is_empty());
}

/// @covers: GrpcStatusCode — distinct variants.
#[test]
fn test_grpc_status_code_ok_is_distinct_from_internal() {
    assert_ne!(GrpcStatusCode::Ok, GrpcStatusCode::Internal);
}

/// @covers: GrpcResponse — struct construction.
#[test]
fn test_grpc_response_holds_body_bytes() {
    let resp = GrpcResponse { body: vec![0x08, 0x01], metadata: GrpcMetadata::default() };
    assert_eq!(resp.body, vec![0x08, 0x01]);
}
