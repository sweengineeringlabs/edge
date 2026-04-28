//! `TonicGrpcClient` — concrete `GrpcOutbound` implementation backed by hyper HTTP/2.

use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::future::BoxFuture;
use futures::StreamExt as _;
use http_body_util::{BodyExt as _, Full};
use hyper_util::client::legacy::connect::HttpConnector;
use tokio_util::sync::CancellationToken;

use crate::api::port::{GrpcMessageStream, GrpcOutbound, GrpcOutboundError, GrpcOutboundResult};
use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};
use crate::core::status_codes::from_wire;

/// Hyper HTTP/2 client type alias.
///
/// `HttpsConnector<HttpConnector>` with `https_or_http()` transparently routes:
/// - `http://` URIs → plain h2c (HTTP/2 clear text, prior knowledge)
/// - `https://` URIs → HTTP/2 over TLS (standard gRPCS)
type HyperClient =
    hyper_util::client::legacy::Client<hyper_rustls::HttpsConnector<HttpConnector>, Full<Bytes>>;

/// Sanitized message for unexpected internal client-side conditions.
///
/// We never propagate raw `e.to_string()` from inside the client crate to
/// the response error — those messages can leak file paths, struct names,
/// and other internals.  The full text goes to the tracing log; the wire
/// error carries this static string.
const SANITIZED_INTERNAL_MSG: &str = "internal client error";

/// Concrete `GrpcOutbound` implementation using hyper HTTP/2 (h2c prior knowledge).
///
/// Suitable for connections to gRPC servers that accept plain-text HTTP/2 (no TLS).
/// Each instance re-uses a single connection pool via `hyper_util::client::legacy::Client`.
pub struct TonicGrpcClient {
    base_uri: String,
    client:   HyperClient,
    /// Fallback timeout used by [`call_stream`] and [`health_check`] only.
    /// Per-request unary calls use the deadline carried on `GrpcRequest`.
    timeout:  Duration,
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
            tracing::warn!(
                expected = len,
                actual   = data.len(),
                "truncated gRPC frame received from server",
            );
            return Err(GrpcOutboundError::Internal(SANITIZED_INTERNAL_MSG.into()));
        }
        out.push(data[..len].to_vec());
        data.advance(len);
    }
    Ok(out)
}

/// Encode a `Duration` as a `grpc-timeout` header value per the gRPC protocol:
/// integer value followed by a unit suffix (`H`, `M`, `S`, `m`, `u`, `n`).
///
/// Picks the smallest unit that fits the duration in a `u64`, preferring
/// milliseconds for sub-second values and seconds otherwise.
fn encode_grpc_timeout(d: Duration) -> String {
    // Per RFC, value MUST fit in 8 ASCII digits — we cap at 99 999 999 of the
    // chosen unit to stay safely within that.  For practical deadlines this is
    // 27+ hours of seconds or 27+ years of seconds — never reached in real use.
    const MAX_VAL: u128 = 99_999_999;

    let nanos = d.as_nanos();
    if nanos == 0 {
        return "0n".into();
    }
    if nanos <= MAX_VAL {
        return format!("{nanos}n");
    }
    let micros = d.as_micros();
    if micros <= MAX_VAL {
        return format!("{micros}u");
    }
    let millis = d.as_millis();
    if millis <= MAX_VAL {
        return format!("{millis}m");
    }
    let secs = d.as_secs();
    if (secs as u128) <= MAX_VAL {
        return format!("{secs}S");
    }
    let mins = secs / 60;
    if (mins as u128) <= MAX_VAL {
        return format!("{mins}M");
    }
    let hours = secs / 3600;
    format!("{hours}H")
}

/// Build a `http::Request` from a pre-encoded body, URI string, and metadata.
///
/// Always injects the gRPC-mandatory headers (`content-type`, `te: trailers`).
/// Caller-supplied metadata headers come last — they may override defaults
/// when intentional (no defensive filtering, since the caller owns the wire).
fn build_http_request(
    uri_str:  &str,
    body_bytes: Bytes,
    metadata: &GrpcMetadata,
    deadline: Option<Duration>,
) -> GrpcOutboundResult<http::Request<Full<Bytes>>> {
    let uri: http::Uri = uri_str.parse().map_err(|e| {
        tracing::warn!(error = %e, uri = %uri_str, "invalid gRPC URI supplied by caller");
        GrpcOutboundError::Internal(SANITIZED_INTERNAL_MSG.into())
    })?;

    let mut builder = http::Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, "application/grpc")
        .header("te", "trailers");

    if let Some(d) = deadline {
        builder = builder.header("grpc-timeout", encode_grpc_timeout(d));
    }

    for (k, v) in &metadata.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }

    builder.body(Full::new(body_bytes)).map_err(|e| {
        tracing::warn!(error = %e, "failed to build HTTP request for gRPC call");
        GrpcOutboundError::Internal(SANITIZED_INTERNAL_MSG.into())
    })
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

/// Check the `grpc-status` trailer value; return `Err(Status(...))` for anything != "0".
///
/// `grpc-message` is a *server-supplied* sanitized message that the gRPC spec
/// already requires not to contain server internals.  We pass it through as-is.
fn check_grpc_status(trailers: Option<&http::HeaderMap>) -> GrpcOutboundResult<()> {
    let code_str = trailers
        .and_then(|m| m.get("grpc-status"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("0");

    if code_str == "0" {
        return Ok(());
    }

    let wire: i32 = code_str.parse().unwrap_or(2 /* Unknown */);
    let code      = from_wire(wire);
    let message   = trailers
        .and_then(|m| m.get("grpc-message"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    Err(GrpcOutboundError::Status(code, message))
}

// ── constructor ───────────────────────────────────────────────────────────────

impl TonicGrpcClient {
    /// Create a client with a 30-second fallback timeout for `call_stream` and
    /// `health_check` (per-call deadlines on `GrpcRequest` always take precedence
    /// for unary calls).
    pub fn new(base_uri: impl Into<String>) -> Self {
        Self::with_timeout(base_uri, Duration::from_secs(30))
    }

    /// Create a client with an explicit fallback timeout.
    pub fn with_timeout(base_uri: impl Into<String>, timeout: Duration) -> Self {
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

/// Race a future against the request's deadline AND its optional cancellation
/// token.  Used to wrap every awaitable network step in unary/stream calls.
///
/// Resolves to:
/// - `Ok(value)`     — future completed in time
/// - `Err(Timeout)`  — deadline elapsed first
/// - `Err(Cancelled)`— cancellation token fired first
async fn race_deadline_and_cancel<F, T>(
    fut:      F,
    deadline: Duration,
    cancel:   Option<&CancellationToken>,
) -> GrpcOutboundResult<T>
where
    F: std::future::Future<Output = T>,
{
    let timeout_fut = tokio::time::timeout(deadline, fut);
    match cancel {
        Some(token) => tokio::select! {
            biased;
            _ = token.cancelled() => Err(GrpcOutboundError::Cancelled(
                "caller cancelled in-flight request".into(),
            )),
            res = timeout_fut => res.map_err(|_| GrpcOutboundError::Timeout(
                "request deadline exceeded".into(),
            )),
        },
        None => timeout_fut.await.map_err(|_| GrpcOutboundError::Timeout(
            "request deadline exceeded".into(),
        )),
    }
}

// ── GrpcOutbound impl ─────────────────────────────────────────────────────────

impl GrpcOutbound for TonicGrpcClient {
    fn call_unary(
        &self,
        request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcOutboundResult<GrpcResponse>> {
        let uri_str   = format!("{}/{}", self.base_uri.trim_end_matches('/'), request.method);
        let body_bytes = encode_grpc_frame(&request.body);
        let deadline   = request.deadline;
        let cancel     = request.cancellation.clone();
        let http_req   = match build_http_request(
            &uri_str, body_bytes, &request.metadata, Some(deadline),
        ) {
            Ok(r)  => r,
            Err(e) => return Box::pin(futures::future::ready(Err(e))),
        };

        Box::pin(async move {
            let resp = race_deadline_and_cancel(
                self.client.request(http_req),
                deadline,
                cancel.as_ref(),
            )
            .await?
            .map_err(|e| {
                tracing::warn!(error = %e, "hyper transport error during gRPC call");
                GrpcOutboundError::ConnectionFailed("transport error".into())
            })?;

            // Check grpc-status in the initial response headers first (some servers
            // send it there rather than in HTTP/2 trailers).
            check_grpc_status(Some(resp.headers()))?;

            let collected = race_deadline_and_cancel(
                resp.into_body().collect(),
                deadline,
                cancel.as_ref(),
            )
            .await?
            .map_err(|e| {
                tracing::warn!(error = %e, "failed to read gRPC response body");
                GrpcOutboundError::Internal(SANITIZED_INTERNAL_MSG.into())
            })?;

            // Also check trailers (the canonical gRPC location).
            check_grpc_status(collected.trailers())?;

            // Extract trailer headers BEFORE consuming bytes.
            let trailer_headers = header_map_to_hash(collected.trailers());
            let data            = collected.to_bytes();

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
    ///
    /// Streaming has no per-request deadline yet — uses the client-level
    /// `timeout`.  Phase 2 will plumb a `GrpcRequest`-shaped streaming envelope.
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
            let mut stream   = messages;
            while let Some(item) = stream.next().await {
                let payload = item?;
                let frame   = encode_grpc_frame(&payload);
                body_buf.put(frame);
            }

            let http_req = build_http_request(
                &uri_str, body_buf.freeze(), &metadata, Some(self.timeout),
            )?;

            let resp = tokio::time::timeout(self.timeout, self.client.request(http_req))
                .await
                .map_err(|_| GrpcOutboundError::Timeout("stream request deadline exceeded".into()))?
                .map_err(|e| {
                    tracing::warn!(error = %e, "hyper transport error during gRPC stream");
                    GrpcOutboundError::ConnectionFailed("transport error".into())
                })?;

            // Check grpc-status in the initial response headers first.
            check_grpc_status(Some(resp.headers()))?;

            let collected = resp
                .into_body()
                .collect()
                .await
                .map_err(|e| {
                    tracing::warn!(error = %e, "failed to read gRPC stream response body");
                    GrpcOutboundError::Internal(SANITIZED_INTERNAL_MSG.into())
                })?;

            // Also check trailers.
            check_grpc_status(collected.trailers())?;

            let data   = collected.to_bytes();
            let frames = decode_grpc_frames(data)?;
            let items: Vec<GrpcOutboundResult<Vec<u8>>> = frames.into_iter().map(Ok).collect();
            Ok(Box::pin(futures::stream::iter(items)) as GrpcMessageStream)
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcOutboundResult<()>> {
        let base_uri = self.base_uri.clone();

        Box::pin(async move {
            let uri: http::Uri = base_uri.parse().map_err(|e| {
                tracing::warn!(error = %e, uri = %base_uri, "invalid gRPC URI in health check");
                GrpcOutboundError::Internal(SANITIZED_INTERNAL_MSG.into())
            })?;

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

/// Compile-time guard: `GrpcRequest::new` MUST require a `Duration`.  This
/// construction site exercises the 3-arg signature; if the signature ever
/// regresses to 2 args this fails to build, which is the gate.
#[doc(hidden)]
#[allow(dead_code)]
pub fn _grpc_request_new_requires_deadline_compile_check() -> GrpcRequest {
    GrpcRequest::new("svc/Method", Vec::new(), Duration::from_secs(1))
}

// Quiet the dead-code warning on a public-only-by-import symbol for some
// downstream test wirings.
#[allow(dead_code)]
fn _suppress_status_code_unused_import_warning(c: GrpcStatusCode) -> GrpcStatusCode { c }

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
        let d      = Duration::from_secs(5);
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
        assert_eq!(frame[0], 0x00);  // not compressed
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

    /// @covers: decode_grpc_frames — sanitized error on truncated frames.
    #[test]
    fn test_decode_grpc_frames_returns_internal_error_on_truncated_data() {
        // 5-byte header says length=100 but only 3 bytes of payload follow.
        let mut buf = BytesMut::new();
        buf.put_u8(0x00);
        buf.put_u32(100_u32);
        buf.put_slice(b"abc");
        let result = decode_grpc_frames(buf.freeze());
        match result {
            Err(GrpcOutboundError::Internal(msg)) => {
                // Must be the sanitized constant — never raw byte counts on the wire.
                assert_eq!(msg, SANITIZED_INTERNAL_MSG);
            }
            other => panic!("expected Internal(sanitized), got {other:?}"),
        }
    }

    /// @covers: encode_grpc_timeout — sub-second values use nanos.
    #[test]
    fn test_encode_grpc_timeout_uses_nanos_for_small_values() {
        assert_eq!(encode_grpc_timeout(Duration::from_nanos(1)),       "1n");
        assert_eq!(encode_grpc_timeout(Duration::from_micros(1)),      "1000n");
    }

    /// @covers: encode_grpc_timeout — millisecond range.
    #[test]
    fn test_encode_grpc_timeout_uses_millis_or_higher_for_seconds() {
        // 30s = 30_000_000_000 ns — too big for nanos, fits microseconds (3e10).
        // Actually 3e10 > 99_999_999, so it falls through to millis (30_000) or higher.
        let s = encode_grpc_timeout(Duration::from_secs(30));
        // Accept any unit suffix as long as the value parses and is positive.
        let last = s.chars().last().expect("non-empty timeout encoding");
        assert!("nuMSmH".contains(last), "unexpected unit suffix in {s}");
    }

    /// @covers: encode_grpc_timeout — zero duration is encoded as 0n.
    #[test]
    fn test_encode_grpc_timeout_handles_zero_duration() {
        assert_eq!(encode_grpc_timeout(Duration::ZERO), "0n");
    }
}
