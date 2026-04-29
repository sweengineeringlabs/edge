//! gRPC outbound client — TonicGrpcClient unary + streaming against an in-process echo server.
//!
//! Run:
//!     cargo run -p swe-edge-egress --example grpc_client
//!
//! Spins up a minimal hyper HTTP/2 echo server in-process, makes a unary
//! call and a streaming call through TonicGrpcClient, then exits.

use std::convert::Infallible;
use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::StreamExt as _;
use http_body_util::{BodyExt as _, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use swe_edge_egress_grpc::{GrpcMessageStream, GrpcMetadata, GrpcOutbound, GrpcRequest, TonicGrpcClient};

// ── gRPC length-prefix frame helpers ─────────────────────────────────────────

fn encode_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0x00); // not compressed
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

// ── in-process gRPC echo server ───────────────────────────────────────────────

async fn start_echo_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind echo server");
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v)  => v,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let _ = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
                    .serve_connection(
                        io,
                        hyper::service::service_fn(
                            |req: http::Request<hyper::body::Incoming>| async {
                                let data =
                                    req.into_body().collect().await.unwrap().to_bytes();
                                let frames = decode_frames(data);
                                let mut buf = BytesMut::new();
                                for f in frames {
                                    buf.put(encode_frame(&f));
                                }
                                Ok::<_, Infallible>(
                                    http::Response::builder()
                                        .status(200)
                                        .header(http::header::CONTENT_TYPE, "application/grpc")
                                        .header("grpc-status", "0")
                                        .body(Full::new(buf.freeze()))
                                        .unwrap(),
                                )
                            },
                        ),
                    )
                    .await;
            });
        }
    });

    format!("http://{addr}")
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // hyper-rustls initialises the TLS stack eagerly even for h2c (http://)
    // connections. With multiple TLS backends in the dependency graph, rustls
    // 0.23 requires an explicit provider selection before any TLS type is
    // constructed. Installing ring here is idempotent — a second call is a no-op.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let uri = start_echo_server().await;
    let client = TonicGrpcClient::new(&uri);

    // Unary call — single request, single response.  Per-call deadline is mandatory.
    let req = GrpcRequest::new(
        "echo.EchoService/Echo",
        b"hello from egress".to_vec(),
        Duration::from_secs(5),
    )
    .with_header("x-request-id", "example-001");
    let resp = client.call_unary(req).await?;
    println!(
        "Unary  → {} bytes echoed: {:?}",
        resp.body.len(),
        std::str::from_utf8(&resp.body).unwrap_or("<binary>")
    );

    // Streaming call — multiple input frames collected into one response stream.
    let messages: GrpcMessageStream = Box::pin(futures::stream::iter(vec![
        Ok(b"frame-1".to_vec()),
        Ok(b"frame-2".to_vec()),
        Ok(b"frame-3".to_vec()),
    ]));
    let mut resp_stream = client
        .call_stream(
            "echo.EchoService/EchoStream".into(),
            GrpcMetadata::default(),
            messages,
        )
        .await?;

    let mut count = 0usize;
    while let Some(item) = resp_stream.next().await {
        let payload = item?;
        println!(
            "Stream [{count}] → {:?}",
            std::str::from_utf8(&payload).unwrap_or("<binary>")
        );
        count += 1;
    }
    println!("Received {count} stream frame(s).");

    Ok(())
}
