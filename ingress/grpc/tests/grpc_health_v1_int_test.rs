//! Phase 3 acceptance integration tests for the standard
//! `grpc.health.v1.Health` service.

use std::net::SocketAddr;
use std::sync::Arc;

use bytes::{BufMut, Bytes, BytesMut};
use http::Request;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use swe_edge_ingress_grpc::{
    HealthService, ServingStatus, TonicGrpcServer, HEALTH_CHECK_METHOD, HEALTH_WATCH_METHOD,
};

fn grpc_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

fn parse_first_grpc_frame(data: &Bytes) -> Option<Bytes> {
    if data.len() < 5 {
        return None;
    }
    let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
    if data.len() < 5 + len {
        return None;
    }
    Some(data.slice(5..5 + len))
}

/// Encode a `HealthCheckRequest { service }` as proto bytes.
fn encode_check_request(service: &str) -> Vec<u8> {
    let bytes = service.as_bytes();
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(2 + bytes.len());
    out.push(0x0a);
    encode_varint(bytes.len() as u64, &mut out);
    out.extend_from_slice(bytes);
    out
}

fn encode_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

/// Decode a `HealthCheckResponse` payload to the `ServingStatus` integer.
fn decode_status(payload: &[u8]) -> i32 {
    if payload.is_empty() {
        return 0; // proto3 default
    }
    if payload[0] != 0x08 {
        return -1;
    }
    let mut result = 0i64;
    let mut shift = 0u32;
    for byte in &payload[1..] {
        result |= ((byte & 0x7f) as i64) << shift;
        if byte & 0x80 == 0 {
            return result as i32;
        }
        shift += 7;
    }
    -1
}

async fn start_health_server(
    health: Arc<HealthService>,
) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let server   = TonicGrpcServer::new("127.0.0.1:0", health);
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
    addr: SocketAddr,
    path: &str,
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

/// @covers: grpc.health.v1.Health.Check — returns SERVING when service is registered SERVING.
#[tokio::test]
async fn test_health_check_returns_serving_for_registered_service_over_grpc_wire() {
    let health = Arc::new(HealthService::new());
    health.set_status("pkg.A", ServingStatus::Serving);
    let (addr, _shutdown) = start_health_server(health).await;

    let (grpc_status, body) =
        grpc_call(addr, HEALTH_CHECK_METHOD, &encode_check_request("pkg.A")).await;
    assert_eq!(grpc_status.as_deref(), Some("0"), "expected grpc-status=0");
    let payload = parse_first_grpc_frame(&body).expect("response frame present");
    assert_eq!(decode_status(&payload), 1, "SERVING == 1");
}

/// @covers: grpc.health.v1.Health.Check — returns NOT_SERVING when service is NOT_SERVING.
#[tokio::test]
async fn test_health_check_returns_not_serving_when_service_marked_not_serving() {
    let health = Arc::new(HealthService::new());
    health.set_status("pkg.A", ServingStatus::NotServing);
    let (addr, _shutdown) = start_health_server(health).await;

    let (grpc_status, body) =
        grpc_call(addr, HEALTH_CHECK_METHOD, &encode_check_request("pkg.A")).await;
    assert_eq!(grpc_status.as_deref(), Some("0"));
    let payload = parse_first_grpc_frame(&body).expect("response frame present");
    assert_eq!(decode_status(&payload), 2, "NOT_SERVING == 2");
}

/// @covers: grpc.health.v1.Health.Check — empty service name returns overall status.
#[tokio::test]
async fn test_health_check_empty_service_returns_overall_status_over_grpc_wire() {
    let health = Arc::new(HealthService::new());
    health.set_overall_status(ServingStatus::Serving);
    let (addr, _shutdown) = start_health_server(health).await;

    let (grpc_status, body) =
        grpc_call(addr, HEALTH_CHECK_METHOD, &encode_check_request("")).await;
    assert_eq!(grpc_status.as_deref(), Some("0"));
    let payload = parse_first_grpc_frame(&body).expect("response frame present");
    assert_eq!(decode_status(&payload), 1, "SERVING == 1");
}

/// @covers: grpc.health.v1.Health.Check — unknown service returns NotFound (5).
#[tokio::test]
async fn test_health_check_returns_not_found_for_unregistered_service_over_grpc_wire() {
    let health = Arc::new(HealthService::new());
    let (addr, _shutdown) = start_health_server(health).await;
    let (grpc_status, _) =
        grpc_call(addr, HEALTH_CHECK_METHOD, &encode_check_request("never.registered")).await;
    // tonic::Code::NotFound == 5
    assert_eq!(grpc_status.as_deref(), Some("5"));
}

/// @covers: grpc.health.v1.Health.Watch — emits initial snapshot then subsequent change.
#[tokio::test]
async fn test_health_watch_streams_initial_snapshot_then_status_change() {
    use futures::StreamExt;
    use http_body_util::BodyStream;

    let health = Arc::new(HealthService::new());
    health.set_status("pkg.A", ServingStatus::Serving);
    let (addr, _shutdown) = start_health_server(health.clone()).await;

    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(io)
        .await
        .unwrap();
    tokio::spawn(conn);

    let req = Request::builder()
        .method("POST")
        .uri(format!("http://{addr}{HEALTH_WATCH_METHOD}"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(Full::new(grpc_frame(&encode_check_request("pkg.A"))))
        .unwrap();

    let resp = sender.send_request(req).await.unwrap();
    let body = resp.into_body();
    let mut frames = BodyStream::new(body);

    // Read the first frame — this is the snapshot.
    let mut first_payload: Option<Vec<u8>> = None;
    while let Some(frame) = frames.next().await {
        let frame = frame.unwrap();
        if let Some(data) = frame.data_ref() {
            if data.len() >= 5 {
                let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
                if data.len() >= 5 + len {
                    first_payload = Some(data[5..5 + len].to_vec());
                    break;
                }
            }
        }
    }
    let payload = first_payload.expect("initial Watch frame");
    assert_eq!(decode_status(&payload), 1, "initial snapshot SERVING == 1");

    // Toggle status — the next frame must be NOT_SERVING.
    health.set_status("pkg.A", ServingStatus::NotServing);

    let mut next_payload: Option<Vec<u8>> = None;
    while let Some(frame) = frames.next().await {
        let frame = frame.unwrap();
        if let Some(data) = frame.data_ref() {
            if data.len() >= 5 {
                let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
                if data.len() >= 5 + len {
                    next_payload = Some(data[5..5 + len].to_vec());
                    break;
                }
            }
        }
        if frame.is_trailers() {
            break;
        }
    }
    let payload = next_payload.expect("status-change Watch frame");
    assert_eq!(decode_status(&payload), 2, "status change NOT_SERVING == 2");
}
