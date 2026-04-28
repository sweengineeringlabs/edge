//! Compression-mode wire test for the egress client.
//!
//! Verifies the toggle works end-to-end: a server records the
//! incoming `grpc-encoding` header and the egress client advertises
//! it correctly when compression is configured.

use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use http_body::Frame;
use http_body_util::{BodyExt as _, StreamBody};

use swe_edge_egress_grpc::{
    CompressionMode, GrpcOutbound, GrpcRequest, TonicGrpcClient,
};

fn encode_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0x00);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

#[tokio::test]
async fn tonic_grpc_client_struct_advertises_grpc_encoding_when_gzip_set_int_test() {
    let recorded: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();

    let recorded_for_server = recorded.clone();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let io = hyper_util::rt::TokioIo::new(stream);
        let svc = hyper::service::service_fn(move |req: http::Request<hyper::body::Incoming>| {
            let recorded = recorded_for_server.clone();
            async move {
                if let Some(v) = req.headers().get("grpc-encoding") {
                    if let Ok(s) = v.to_str() {
                        *recorded.lock().unwrap() = Some(s.to_string());
                    }
                }
                let _ = req.collect().await;
                let mut trailers = http::HeaderMap::new();
                trailers.insert("grpc-status", http::HeaderValue::from_static("0"));
                let frames: Vec<Result<Frame<Bytes>, Infallible>> = vec![
                    Ok(Frame::data(encode_frame(b"ok"))),
                    Ok(Frame::trailers(trailers)),
                ];
                let body = StreamBody::new(futures::stream::iter(frames)).boxed();
                Ok::<_, Infallible>(
                    http::Response::builder()
                        .status(200)
                        .header("content-type", "application/grpc")
                        .body(body)
                        .unwrap(),
                )
            }
        });
        let _ = hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
            .serve_connection(io, svc)
            .await;
    });
    tokio::time::sleep(Duration::from_millis(20)).await;

    let client = TonicGrpcClient::new(format!("http://{addr}"))
        .with_compression(CompressionMode::Gzip);
    let req = GrpcRequest::new("svc/M", b"payload".to_vec(), Duration::from_secs(2));
    let _ = client.call_unary(req).await.expect("call_unary");

    let value = recorded.lock().unwrap().clone();
    assert_eq!(
        value.as_deref(),
        Some("gzip"),
        "client must advertise grpc-encoding=gzip when CompressionMode::Gzip is set",
    );
}

#[tokio::test]
async fn tonic_grpc_client_struct_does_not_set_grpc_encoding_when_none_int_test() {
    let recorded: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();

    let recorded_for_server = recorded.clone();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let io = hyper_util::rt::TokioIo::new(stream);
        let svc = hyper::service::service_fn(move |req: http::Request<hyper::body::Incoming>| {
            let recorded = recorded_for_server.clone();
            async move {
                if let Some(v) = req.headers().get("grpc-encoding") {
                    if let Ok(s) = v.to_str() {
                        *recorded.lock().unwrap() = Some(s.to_string());
                    }
                }
                let _ = req.collect().await;
                let mut trailers = http::HeaderMap::new();
                trailers.insert("grpc-status", http::HeaderValue::from_static("0"));
                let frames: Vec<Result<Frame<Bytes>, Infallible>> = vec![
                    Ok(Frame::data(encode_frame(b"ok"))),
                    Ok(Frame::trailers(trailers)),
                ];
                let body = StreamBody::new(futures::stream::iter(frames)).boxed();
                Ok::<_, Infallible>(
                    http::Response::builder()
                        .status(200)
                        .header("content-type", "application/grpc")
                        .body(body)
                        .unwrap(),
                )
            }
        });
        let _ = hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
            .serve_connection(io, svc)
            .await;
    });
    tokio::time::sleep(Duration::from_millis(20)).await;

    let client = TonicGrpcClient::new(format!("http://{addr}"));
    let req = GrpcRequest::new("svc/M", b"payload".to_vec(), Duration::from_secs(2));
    let _ = client.call_unary(req).await.expect("call_unary");

    let value = recorded.lock().unwrap().clone();
    assert!(
        value.is_none(),
        "client must NOT set grpc-encoding when compression is None (got {:?})",
        value,
    );
}
