//! Integration tests for `TonicGrpcClient` / `GrpcOutbound`.
//!
//! Tests spin up a minimal in-process HTTP/2 echo server (no external process).

use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::stream;
use http_body::Frame;
use http_body_util::{BodyExt as _, Full, StreamBody};

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

/// Spawn a gRPC server that accepts HTTP/2 connections but never sends a response.
/// Used to verify that the client's timeout fires correctly.
async fn spawn_stalling_grpc_server(listener: tokio::net::TcpListener) -> SocketAddr {
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
                        // Complete the HTTP/2 handshake but never return a response.
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        Ok::<_, Infallible>(
                            http::Response::builder()
                                .status(200)
                                .body(http_body_util::Full::new(bytes::Bytes::new()))
                                .unwrap(),
                        )
                    }),
                )
                .await;
            });
        }
    });

    addr
}

/// Spawn a server that echoes request frames back and sends custom metadata as HTTP/2 trailers.
async fn spawn_metadata_server(listener: tokio::net::TcpListener) -> SocketAddr {
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v)  => v,
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
                        let collected = req.into_body().collect().await.unwrap();
                        let body_bytes = collected.to_bytes();
                        let frames = decode_frames(body_bytes);
                        let mut resp_buf = BytesMut::new();
                        for f in &frames {
                            resp_buf.put(encode_frame(f));
                        }

                        let mut trailers = http::HeaderMap::new();
                        trailers.insert("grpc-status",   http::HeaderValue::from_static("0"));
                        trailers.insert("x-response-id", http::HeaderValue::from_static("meta-42"));

                        let body = StreamBody::new(futures::stream::iter(vec![
                            Ok::<Frame<Bytes>, Infallible>(Frame::data(resp_buf.freeze())),
                            Ok(Frame::trailers(trailers)),
                        ]));

                        let resp = http::Response::builder()
                            .status(200)
                            .header(http::header::CONTENT_TYPE, "application/grpc")
                            .body(body.boxed())
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

/// Install a rustls CryptoProvider exactly once per test process.  In a
/// workspace where multiple crates pull both `aws-lc-rs` and `ring`, rustls
/// 0.23 refuses to auto-select; tests that construct a `TonicGrpcClient`
/// must call this first so hyper-rustls can build its connector.
fn ensure_rustls_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// @covers: TonicGrpcClient::call_unary — happy path echo.
#[tokio::test]
async fn test_call_unary_sends_request_and_receives_response() {
    ensure_rustls_provider();
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let req = GrpcRequest::new("echo/Echo", b"hello".to_vec(), Duration::from_secs(5));

    let resp = client.call_unary(req).await.expect("call_unary should succeed");
    assert_eq!(resp.body, b"hello");
}

/// @covers: TonicGrpcClient::call_unary — grpc-status 13 maps to Status(Internal, _).
#[tokio::test]
async fn test_call_unary_propagates_grpc_error_status() {
    let listener = bind_listener().await;
    let addr = spawn_error_server(listener).await;

    ensure_rustls_provider();
    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let req = GrpcRequest::new("echo/Echo", b"ping".to_vec(), Duration::from_secs(5));

    let result = client.call_unary(req).await;
    assert!(result.is_err(), "expected Err for grpc-status 13, got Ok");
    match result.unwrap_err() {
        GrpcOutboundError::Status(GrpcStatusCode::Internal, msg) => {
            // server-side message comes through grpc-message verbatim
            assert!(
                msg.contains("server-side error"),
                "message should match grpc-message header, got: {msg}"
            );
        }
        other => panic!("expected Status(Internal, _), got {other:?}"),
    }
}

/// @covers: TonicGrpcClient::call_stream — multiple frames echoed back.
#[tokio::test]
async fn test_call_stream_sends_multiple_frames_and_receives_stream() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
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

    ensure_rustls_provider();
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

    ensure_rustls_provider();
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

/// @covers: GrpcRequest::new — three-argument construction with deadline.
#[test]
fn test_grpc_request_holds_method_and_body() {
    let req = GrpcRequest::new("svc/Method", vec![1, 2, 3], Duration::from_secs(1));
    assert_eq!(req.method,   "svc/Method");
    assert_eq!(req.body,     vec![1, 2, 3]);
    assert_eq!(req.deadline, Duration::from_secs(1));
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

/// @covers: TonicGrpcClient::call_unary — response metadata from HTTP/2 trailers is preserved.
#[tokio::test]
async fn test_call_unary_receives_response_metadata_from_trailers() {
    let listener = bind_listener().await;
    let addr = spawn_metadata_server(listener).await;

    ensure_rustls_provider();
    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let req = GrpcRequest::new("echo/Echo", b"hi".to_vec(), Duration::from_secs(5));

    let resp = client.call_unary(req).await.expect("call_unary should succeed");
    assert_eq!(resp.body, b"hi", "body must be echoed");
    assert_eq!(
        resp.metadata.headers.get("x-response-id").map(String::as_str),
        Some("meta-42"),
        "x-response-id trailer must be present in response metadata; got: {:?}",
        resp.metadata.headers
    );
}

/// @covers: TonicGrpcClient::call_unary — timeout fires when server does not respond.
#[tokio::test]
async fn test_call_unary_returns_timeout_error_when_server_stalls() {
    let listener = bind_listener().await;
    let addr = spawn_stalling_grpc_server(listener).await;

    // Per-call deadline drives the timeout for unary now.
    ensure_rustls_provider();
    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let req = GrpcRequest::new("svc/Method", b"ping".to_vec(), Duration::from_millis(80));
    let result = client.call_unary(req).await;
    assert!(
        matches!(result, Err(GrpcOutboundError::Timeout(_))),
        "expected Timeout, got {result:?}"
    );
}

/// @covers: TonicGrpcClient::call_unary — ConnectionFailed when nothing listens on the port.
#[tokio::test]
async fn test_call_unary_returns_connection_failed_when_no_server_is_listening() {
    let addr = {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let a = l.local_addr().unwrap();
        drop(l);
        a
    };

    ensure_rustls_provider();
    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let req = GrpcRequest::new("svc/Method", b"ping".to_vec(), Duration::from_secs(5));
    let result = client.call_unary(req).await;
    assert!(
        matches!(result, Err(GrpcOutboundError::ConnectionFailed(_))),
        "expected ConnectionFailed, got {result:?}"
    );
}

// ── Phase 1 enrichment: deadlines, cancellation, status mapping ──────────────

use std::sync::{Arc, Mutex};

/// Echo server that records the `grpc-timeout` header value of the most recent
/// request, so tests can assert that the egress side really sent it.
async fn spawn_timeout_recording_server(
    listener: tokio::net::TcpListener,
) -> (SocketAddr, Arc<Mutex<Option<String>>>) {
    let addr     = listener.local_addr().expect("local_addr");
    let captured = Arc::new(Mutex::new(None::<String>));
    let cap      = captured.clone();
    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v)  => v,
                Err(_) => break,
            };
            let cap = cap.clone();
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let _ = hyper::server::conn::http2::Builder::new(
                    hyper_util::rt::TokioExecutor::new(),
                )
                .serve_connection(
                    io,
                    hyper::service::service_fn(
                        move |req: http::Request<hyper::body::Incoming>| {
                            let cap = cap.clone();
                            async move {
                                if let Some(v) = req.headers().get("grpc-timeout") {
                                    if let Ok(s) = v.to_str() {
                                        *cap.lock().unwrap() = Some(s.to_owned());
                                    }
                                }
                                let collected = req.into_body().collect().await.unwrap();
                                let body_bytes = collected.to_bytes();
                                let frames     = decode_frames(body_bytes);
                                let mut buf    = BytesMut::new();
                                for f in frames {
                                    buf.put(encode_frame(&f));
                                }
                                let resp = http::Response::builder()
                                    .status(200)
                                    .header(http::header::CONTENT_TYPE, "application/grpc")
                                    .header("grpc-status", "0")
                                    .body(Full::new(buf.freeze()))
                                    .unwrap();
                                Ok::<_, Infallible>(resp)
                            }
                        },
                    ),
                )
                .await;
            });
        }
    });
    (addr, captured)
}

/// @covers: TonicGrpcClient::call_unary — request carries `grpc-timeout` header
/// derived from the GrpcRequest deadline.
#[tokio::test]
async fn test_call_unary_sends_grpc_timeout_header_from_deadline() {
    let listener        = bind_listener().await;
    let (addr, captured) = spawn_timeout_recording_server(listener).await;

    ensure_rustls_provider();
    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let req    = GrpcRequest::new("svc/Method", b"x".to_vec(), Duration::from_secs(2));
    client.call_unary(req).await.expect("call_unary should succeed");

    let header = captured.lock().unwrap().clone();
    let header = header.expect("server must have observed a grpc-timeout header");
    // The encoder may pick any unit; just verify it ends with one of the standard suffixes
    // and parses to a non-empty positive value.
    let last = header.chars().last().expect("non-empty header");
    assert!("nuMSmH".contains(last), "unexpected unit suffix in {header}");
    let prefix = &header[..header.len() - 1];
    let value: u64 = prefix.parse().unwrap_or(0);
    assert!(value > 0, "grpc-timeout value must be positive, got {header}");
}

/// @covers: TonicGrpcClient::call_unary — caller-supplied cancellation token aborts
/// the in-flight request and yields `Cancelled` rather than `Timeout`.
#[tokio::test]
async fn test_call_unary_cancellation_token_aborts_in_flight_request() {
    use tokio_util::sync::CancellationToken;

    let listener = bind_listener().await;
    let addr     = spawn_stalling_grpc_server(listener).await;

    // Long deadline so timeout doesn't beat us; cancellation must win.
    ensure_rustls_provider();
    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let token  = CancellationToken::new();
    let req    = GrpcRequest::new("svc/Method", b"x".to_vec(), Duration::from_secs(60))
        .with_cancellation(token.clone());

    // Fire the cancel after a brief wait so the request is actually in flight.
    let cancel_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        token.cancel();
    });

    let result = client.call_unary(req).await;
    cancel_handle.await.unwrap();

    match result {
        Err(GrpcOutboundError::Cancelled(msg)) => {
            assert!(
                msg.to_lowercase().contains("cancel"),
                "Cancelled message should mention cancellation, got: {msg}"
            );
        }
        other => panic!("expected Cancelled, got {other:?}"),
    }
}

/// @covers: GrpcOutboundError::Status — all 17 GrpcStatusCode variants round-trip
/// through the public error type without information loss.
#[test]
fn test_status_error_round_trips_all_17_grpc_status_code_variants() {
    let all_17 = [
        GrpcStatusCode::Ok,
        GrpcStatusCode::Cancelled,
        GrpcStatusCode::Unknown,
        GrpcStatusCode::InvalidArgument,
        GrpcStatusCode::DeadlineExceeded,
        GrpcStatusCode::NotFound,
        GrpcStatusCode::AlreadyExists,
        GrpcStatusCode::PermissionDenied,
        GrpcStatusCode::ResourceExhausted,
        GrpcStatusCode::FailedPrecondition,
        GrpcStatusCode::Aborted,
        GrpcStatusCode::OutOfRange,
        GrpcStatusCode::Unimplemented,
        GrpcStatusCode::Internal,
        GrpcStatusCode::Unavailable,
        GrpcStatusCode::DataLoss,
        GrpcStatusCode::Unauthenticated,
    ];
    assert_eq!(all_17.len(), 17);
    for code in all_17 {
        let err = GrpcOutboundError::Status(code, "msg".into());
        match err {
            GrpcOutboundError::Status(c, m) => {
                assert_eq!(c, code, "code lost in round trip for {code:?}");
                assert_eq!(m, "msg");
            }
            other => panic!("expected Status, got {other:?}"),
        }
    }
}

/// @covers: TonicGrpcClient::call_unary — sanitized message is on the wire when
/// the local client encounters an unexpected internal condition.  We inject one
/// by feeding a server that returns a malformed (truncated) gRPC frame in the
/// response body.
#[tokio::test]
async fn test_call_unary_sanitizes_internal_error_message_on_truncated_response() {
    use std::convert::Infallible;
    let listener = bind_listener().await;
    let addr     = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let io = hyper_util::rt::TokioIo::new(stream);
        let _ = hyper::server::conn::http2::Builder::new(
            hyper_util::rt::TokioExecutor::new(),
        )
        .serve_connection(
            io,
            hyper::service::service_fn(|_req: http::Request<hyper::body::Incoming>| async {
                // Frame header claims length=100 but only 3 bytes follow → truncated frame.
                let mut buf = BytesMut::new();
                buf.put_u8(0x00);
                buf.put_u32(100);
                buf.put_slice(b"abc");
                let resp = http::Response::builder()
                    .status(200)
                    .header(http::header::CONTENT_TYPE, "application/grpc")
                    .header("grpc-status", "0")
                    .body(Full::new(buf.freeze()))
                    .unwrap();
                Ok::<_, Infallible>(resp)
            }),
        )
        .await;
    });

    ensure_rustls_provider();
    let client = TonicGrpcClient::new(format!("http://{addr}"));
    // call_stream is the path that decodes frames and surfaces the truncation.
    let messages: GrpcMessageStream = Box::pin(stream::iter(vec![Ok(b"x".to_vec())]));
    let result = client
        .call_stream("svc/Method".into(), GrpcMetadata::default(), messages)
        .await;
    let err = match result {
        Ok(_)  => panic!("expected an error from a truncated server response, got Ok"),
        Err(e) => e,
    };
    match err {
        GrpcOutboundError::Internal(msg) => {
            // Sanitized: must NOT contain raw byte counts, file names, or struct names.
            assert!(
                !msg.contains("expected") && !msg.contains("got"),
                "internal error message leaked details: {msg}"
            );
            assert!(!msg.is_empty(), "sanitized message must be non-empty");
        }
        other => panic!("expected sanitized Internal error, got {other:?}"),
    }
}
