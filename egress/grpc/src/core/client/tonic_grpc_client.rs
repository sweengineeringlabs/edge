//! `TonicGrpcClient` — concrete `GrpcOutbound` implementation backed by hyper HTTP/2.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::future::BoxFuture;
use futures::StreamExt as _;
use http_body_util::{BodyExt as _, Full};
use hyper_util::client::legacy::connect::HttpConnector;

use crate::api::port::{GrpcMessageStream, GrpcOutbound, GrpcOutboundError, GrpcOutboundResult};
use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse};

/// Hyper HTTP/2 client type alias.
///
/// `HttpsConnector<HttpConnector>` with `https_or_http()` transparently routes:
/// - `http://` URIs → plain h2c (HTTP/2 clear text, prior knowledge)
/// - `https://` URIs → HTTP/2 over TLS (standard gRPCS)
type HyperClient =
    hyper_util::client::legacy::Client<hyper_rustls::HttpsConnector<HttpConnector>, Full<Bytes>>;

/// Concrete `GrpcOutbound` implementation using hyper HTTP/2 (h2c prior knowledge).
///
/// Suitable for connections to gRPC servers that accept plain-text HTTP/2 (no TLS).
/// Each instance re-uses a single connection pool via `hyper_util::client::legacy::Client`.
pub struct TonicGrpcClient {
    base_uri: String,
    client: HyperClient,
    timeout: std::time::Duration,
}

// ── internal helpers ─────────────────────────────────────────────────────────

/// Encode `payload` as a single gRPC data frame:
/// `[0x00][len_u32_be][payload]` — not compressed.
fn encode_grpc_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0x00); // compression flag: not compressed
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

/// Decode all complete gRPC frames from `data`.
///
/// Each frame: `[compression_flag: u8][length: u32_be][payload: length bytes]`.
/// Frames with the compression flag set are passed through as-is (no decompression).
fn decode_grpc_frames(mut data: Bytes) -> GrpcOutboundResult<Vec<Vec<u8>>> {
    const FRAME_HEADER: usize = 5;
    let mut out = Vec::new();
    while data.len() >= FRAME_HEADER {
        let _flag = data[0];
        let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
        data.advance(FRAME_HEADER);
        if data.len() < len {
            return Err(GrpcOutboundError::Internal(format!(
                "truncated gRPC frame: expected {len} bytes, got {}",
                data.len()
            )));
        }
        out.push(data[..len].to_vec());
        data.advance(len);
    }
    Ok(out)
}

/// Build a `http::Request` from a pre-encoded body, URI string, and metadata.
fn build_http_request(
    uri_str: &str,
    body_bytes: Bytes,
    metadata: &GrpcMetadata,
) -> GrpcOutboundResult<http::Request<Full<Bytes>>> {
    let uri: http::Uri = uri_str
        .parse()
        .map_err(|e| GrpcOutboundError::Internal(format!("invalid URI `{uri_str}`: {e}")))?;

    let mut builder = http::Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, "application/grpc")
        .header("te", "trailers");

    for (k, v) in &metadata.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }

    builder
        .body(Full::new(body_bytes))
        .map_err(|e| GrpcOutboundError::Internal(format!("failed to build request: {e}")))
}

/// Extract `HashMap<String, String>` from an optional `HeaderMap` reference.
fn header_map_to_hash(map: Option<&http::HeaderMap>) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    if let Some(m) = map {
        for (k, v) in m {
            if let Ok(s) = v.to_str() {
                out.insert(k.as_str().to_owned(), s.to_owned());
            }
        }
    }
    out
}

/// Check the `grpc-status` trailer value; return `Err` for anything != "0".
fn check_grpc_status(
    trailers: Option<&http::HeaderMap>,
) -> GrpcOutboundResult<()> {
    let code = trailers
        .and_then(|m| m.get("grpc-status"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("0");

    if code != "0" {
        let message = trailers
            .and_then(|m| m.get("grpc-message"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        return Err(GrpcOutboundError::Internal(format!(
            "grpc-status {code}: {message}"
        )));
    }
    Ok(())
}

// ── constructor ───────────────────────────────────────────────────────────────

impl TonicGrpcClient {
    /// Create a client with the default 30-second timeout.
    pub fn new(base_uri: impl Into<String>) -> Self {
        Self::with_timeout(base_uri, std::time::Duration::from_secs(30))
    }

    /// Create a client with an explicit request timeout.
    pub fn with_timeout(base_uri: impl Into<String>, timeout: std::time::Duration) -> Self {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http2()
            .build();
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .http2_only(true)
                .build(connector);
        Self { base_uri: base_uri.into(), client, timeout }
    }
}

// ── GrpcOutbound impl ─────────────────────────────────────────────────────────

impl GrpcOutbound for TonicGrpcClient {
    fn call_unary(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcOutboundResult<GrpcResponse>> {
        let uri_str =
            format!("{}/{}", self.base_uri.trim_end_matches('/'), request.method);
        let body_bytes = encode_grpc_frame(&request.body);
        let http_req = match build_http_request(&uri_str, body_bytes, &request.metadata) {
            Ok(r) => r,
            Err(e) => return Box::pin(futures::future::ready(Err(e))),
        };

        Box::pin(async move {
            let resp = tokio::time::timeout(self.timeout, self.client.request(http_req))
                .await
                .map_err(|_| GrpcOutboundError::Timeout("request timed out".into()))?
                .map_err(|e| GrpcOutboundError::ConnectionFailed(e.to_string()))?;

            // Check grpc-status in the initial response headers first (some servers
            // send it there rather than in HTTP/2 trailers).
            check_grpc_status(Some(resp.headers()))?;

            let collected = resp
                .into_body()
                .collect()
                .await
                .map_err(|e| GrpcOutboundError::Internal(e.to_string()))?;

            // Also check trailers (the canonical gRPC location).
            check_grpc_status(collected.trailers())?;

            // Extract trailer headers BEFORE consuming bytes.
            let trailer_headers = header_map_to_hash(collected.trailers());
            let data = collected.to_bytes();

            // Strip 5-byte gRPC frame header from the response payload.
            let body = if data.len() >= 5 {
                data[5..].to_vec()
            } else {
                data.to_vec()
            };

            Ok(GrpcResponse {
                body,
                metadata: GrpcMetadata { headers: trailer_headers },
            })
        })
    }

    /// Send a gRPC streaming call.
    ///
    /// **Buffering limitation**: both the request and response are fully buffered
    /// in memory before this future resolves. The `GrpcMessageStream` input is
    /// collected into a single HTTP/2 DATA frame sequence, and the response body
    /// is collected before returning. This is functionally correct for small
    /// message sets but is not suitable for large or infinite streams. True
    /// chunked streaming requires replacing hyper's `Full<Bytes>` body type with
    /// a streaming body, which is a separate task.
    fn call_stream(
        &self,
        method: String,
        metadata: GrpcMetadata,
        messages: GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcOutboundResult<GrpcMessageStream>> {
        let uri_str = format!("{}/{}", self.base_uri.trim_end_matches('/'), method);

        Box::pin(async move {
            // Collect all input messages into one body (multiple gRPC frames).
            let mut body_buf = BytesMut::new();
            let mut stream = messages;
            while let Some(item) = stream.next().await {
                let payload = item?;
                let frame = encode_grpc_frame(&payload);
                body_buf.put(frame);
            }

            let http_req =
                build_http_request(&uri_str, body_buf.freeze(), &metadata)?;

            let resp = tokio::time::timeout(self.timeout, self.client.request(http_req))
                .await
                .map_err(|_| GrpcOutboundError::Timeout("stream request timed out".into()))?
                .map_err(|e| GrpcOutboundError::ConnectionFailed(e.to_string()))?;

            // Check grpc-status in the initial response headers first.
            check_grpc_status(Some(resp.headers()))?;

            let collected = resp
                .into_body()
                .collect()
                .await
                .map_err(|e| GrpcOutboundError::Internal(e.to_string()))?;

            // Also check trailers.
            check_grpc_status(collected.trailers())?;

            let data = collected.to_bytes();
            let frames = decode_grpc_frames(data)?;
            let items: Vec<GrpcOutboundResult<Vec<u8>>> = frames.into_iter().map(Ok).collect();
            Ok(Box::pin(futures::stream::iter(items)) as GrpcMessageStream)
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcOutboundResult<()>> {
        let base_uri = self.base_uri.clone();

        Box::pin(async move {
            let uri: http::Uri = base_uri
                .parse()
                .map_err(|e| GrpcOutboundError::Internal(format!("invalid URI: {e}")))?;

            let host = uri.host().unwrap_or("127.0.0.1").to_owned();
            let port = uri.port_u16().unwrap_or(50051);
            let addr = format!("{host}:{port}");

            tokio::net::TcpStream::connect(&addr)
                .await
                .map(|_| ())
                .map_err(|e| GrpcOutboundError::Unavailable(format!("{addr}: {e}")))
        })
    }
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_client_stores_base_uri() {
        rustls::crypto::aws_lc_rs::default_provider().install_default().ok();
        let client = TonicGrpcClient::new("http://localhost:50051");
        assert_eq!(client.base_uri, "http://localhost:50051");
    }

    #[test]
    fn test_with_timeout_overrides_duration() {
        rustls::crypto::aws_lc_rs::default_provider().install_default().ok();
        let d = std::time::Duration::from_secs(5);
        let client = TonicGrpcClient::with_timeout("http://localhost:50051", d);
        assert_eq!(client.timeout, d);
    }

    #[test]
    fn test_grpc_outbound_is_object_safe() {
        fn _assert(_: &dyn GrpcOutbound) {}
    }

    #[test]
    fn test_encode_grpc_frame_produces_5_byte_header() {
        let frame = encode_grpc_frame(b"hello");
        assert_eq!(frame.len(), 10); // 5 header + 5 payload
        assert_eq!(frame[0], 0x00); // not compressed
        assert_eq!(u32::from_be_bytes([frame[1], frame[2], frame[3], frame[4]]), 5);
        assert_eq!(&frame[5..], b"hello");
    }

    #[test]
    fn test_decode_grpc_frames_round_trips_single_frame() {
        let encoded = encode_grpc_frame(b"world");
        let decoded = decode_grpc_frames(encoded).expect("decode failed");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], b"world");
    }

    #[test]
    fn test_decode_grpc_frames_round_trips_multiple_frames() {
        let mut buf = BytesMut::new();
        buf.put(encode_grpc_frame(b"one"));
        buf.put(encode_grpc_frame(b"two"));
        buf.put(encode_grpc_frame(b"three"));
        let decoded = decode_grpc_frames(buf.freeze()).expect("decode failed");
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0], b"one");
        assert_eq!(decoded[1], b"two");
        assert_eq!(decoded[2], b"three");
    }

    #[test]
    fn test_decode_grpc_frames_returns_error_on_truncated_data() {
        // 5-byte header says length=100 but only 3 bytes of payload follow
        let mut buf = BytesMut::new();
        buf.put_u8(0x00);
        buf.put_u32(100_u32);
        buf.put_slice(b"abc");
        let result = decode_grpc_frames(buf.freeze());
        assert!(result.is_err());
    }
}
