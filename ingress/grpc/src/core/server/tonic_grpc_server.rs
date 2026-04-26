//! gRPC server — binds a socket and delegates all unary calls to a
//! [`GrpcInbound`] handler.  HTTP/2 framing is handled by Hyper directly
//! (avoiding the axum 0.7 / 0.8 type mismatch that Tonic's routing layer
//! would otherwise introduce). gRPC length-prefix framing is handled here.

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::sync::Arc;

use bytes::{BufMut, Bytes, BytesMut};
use http::{Request, Response};
use http_body_util::{BodyExt, Limited, StreamBody};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;

use crate::api::port::grpc_inbound::{GrpcInbound, GrpcInboundError, GrpcMessageStream};
use crate::api::value_object::GrpcMetadata;

/// Hard cap on incoming message size.
pub const MAX_MESSAGE_BYTES: usize = 4 * 1_024 * 1_024; // 4 MiB

type BoxBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

/// Error returned by [`TonicGrpcServer::serve`].
#[derive(Debug, thiserror::Error)]
pub enum TonicServerError {
    #[error("failed to bind to {0}: {1}")]
    Bind(String, #[source] std::io::Error),
}

/// gRPC server that routes all unary requests through a [`GrpcInbound`] port.
///
/// No proto-gen is required — raw bytes flow directly to the port.
/// Consumers can swap the handler (override) or wrap it in a decorator
/// that also implements [`GrpcInbound`] to add auth, tracing, etc. (extend).
pub struct TonicGrpcServer {
    bind:      String,
    handler:   Arc<dyn GrpcInbound>,
    max_bytes: usize,
}

impl TonicGrpcServer {
    /// Create a server that will bind to `bind` and delegate to `handler`.
    pub fn new(bind: impl Into<String>, handler: Arc<dyn GrpcInbound>) -> Self {
        Self { bind: bind.into(), handler, max_bytes: MAX_MESSAGE_BYTES }
    }

    /// Override the maximum incoming message size (default: [`MAX_MESSAGE_BYTES`]).
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_bytes = size;
        self
    }

    /// Bind and serve until `shutdown` resolves.
    pub async fn serve<F>(&self, shutdown: F) -> Result<(), TonicServerError>
    where
        F: Future<Output = ()>,
    {
        let listener = TcpListener::bind(&self.bind)
            .await
            .map_err(|e| TonicServerError::Bind(self.bind.clone(), e))?;
        self.serve_with_listener(listener, shutdown).await
    }

    /// Serve using a caller-supplied pre-bound listener.
    ///
    /// Useful for port-0 allocation in tests or pre-bind for zero-downtime
    /// restarts — consistent with the HTTP server pattern.
    pub async fn serve_with_listener<F>(
        &self,
        listener: TcpListener,
        shutdown: F,
    ) -> Result<(), TonicServerError>
    where
        F: Future<Output = ()>,
    {
        tracing::info!(
            bind = %listener.local_addr().map(|a| a.to_string()).unwrap_or_else(|_| self.bind.clone()),
            "gRPC server listening"
        );

        let handler   = self.handler.clone();
        let max_bytes = self.max_bytes;
        let mut shutdown = std::pin::pin!(shutdown);

        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (stream, _) = match res {
                        Ok(s)  => s,
                        Err(e) => { tracing::warn!("gRPC accept error: {e}"); continue; }
                    };
                    let handler = handler.clone();
                    tokio::spawn(async move {
                        let svc = hyper::service::service_fn(move |req| {
                            let handler = handler.clone();
                            async move {
                                Ok::<_, Infallible>(dispatch(req, handler, max_bytes).await)
                            }
                        });
                        let io = TokioIo::new(stream);
                        if let Err(e) = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
                            .serve_connection(io, svc)
                            .await
                        {
                            tracing::debug!("gRPC connection error: {e}");
                        }
                    });
                }
                _ = &mut shutdown => break,
            }
        }

        Ok(())
    }
}

// ── gRPC dispatch ─────────────────────────────────────────────────────────────

async fn dispatch(
    req:       Request<hyper::body::Incoming>,
    handler:   Arc<dyn GrpcInbound>,
    max_bytes: usize,
) -> Response<BoxBody> {
    let method   = req.uri().path().to_string();
    let metadata = collect_metadata(req.headers());

    let body_bytes = match Limited::new(req.into_body(), max_bytes).collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_)        => return grpc_error(tonic::Code::ResourceExhausted, "message too large"),
    };

    // Decode all gRPC length-prefix frames from the body.
    let frames = decode_grpc_frames(&body_bytes);
    let message_stream: GrpcMessageStream = Box::pin(futures::stream::iter(
        frames.into_iter().map(|f| Ok::<Vec<u8>, GrpcInboundError>(f.to_vec())),
    ));

    match handler.handle_stream(method, GrpcMetadata { headers: metadata }, message_stream).await {
        Ok((resp_stream, resp_meta)) => grpc_stream_response(resp_stream, resp_meta).await,
        Err(e)          => {
            let (code, msg) = map_error(e);
            grpc_error(code, msg)
        }
    }
}

/// Parse all gRPC length-prefix frames from a raw body.
///
/// Each frame: 1 byte compressed flag + 4 bytes big-endian message length + payload.
/// Frames with a compressed flag != 0 are still yielded (payload returned as-is).
fn decode_grpc_frames(data: &Bytes) -> Vec<Bytes> {
    const HEADER: usize = 5;
    let mut frames = Vec::new();
    let mut offset = 0usize;
    while offset + HEADER <= data.len() {
        // bytes 1-4 are the big-endian message length; byte 0 is the compressed flag.
        let len = u32::from_be_bytes([
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
        ]) as usize;
        let payload_start = offset + HEADER;
        let payload_end   = payload_start + len;
        if payload_end > data.len() {
            break; // truncated — stop rather than panic
        }
        frames.push(data.slice(payload_start..payload_end));
        offset = payload_end;
    }
    // If no valid frame header was found treat the entire body as one raw payload
    // (handles the degenerate case of an empty or header-only body gracefully).
    if frames.is_empty() && !data.is_empty() {
        frames.push(data.clone());
    }
    frames
}

/// Collect a response stream into a single HTTP/2 response with one DATA frame
/// per stream item plus a trailing `grpc-status=0` header and any response metadata.
async fn grpc_stream_response(mut stream: GrpcMessageStream, meta: GrpcMetadata) -> Response<BoxBody> {
    use futures::StreamExt;

    // Collect all response messages.
    let mut frames: Vec<Bytes> = Vec::new();
    loop {
        match stream.next().await {
            Some(Ok(payload)) => {
                let mut buf = BytesMut::with_capacity(5 + payload.len());
                buf.put_u8(0); // not compressed
                buf.put_u32(payload.len() as u32);
                buf.put_slice(&payload);
                frames.push(buf.freeze());
            }
            Some(Err(e)) => {
                let (code, msg) = map_error(e);
                return grpc_error(code, msg);
            }
            None => break,
        }
    }

    let mut trailers = http::HeaderMap::new();
    trailers.insert("grpc-status", http::HeaderValue::from_static("0"));
    for (k, v) in &meta.headers {
        if let (Ok(name), Ok(val)) = (
            http::HeaderName::from_bytes(k.as_bytes()),
            http::HeaderValue::from_str(v),
        ) {
            trailers.insert(name, val);
        }
    }

    // Build the response body: one DATA frame per response message, then trailers.
    let mut http_frames: Vec<Result<http_body::Frame<Bytes>, Infallible>> =
        frames.into_iter().map(|b| Ok(http_body::Frame::data(b))).collect();
    http_frames.push(Ok(http_body::Frame::trailers(trailers)));

    let response_body = BodyExt::boxed(StreamBody::new(futures::stream::iter(http_frames)));

    Response::builder()
        .status(200)
        .header("content-type", "application/grpc")
        .body(response_body)
        .unwrap()
}

// ── gRPC framing helpers ──────────────────────────────────────────────────────

fn grpc_error(code: tonic::Code, message: impl Into<String>) -> Response<BoxBody> {
    let message = message.into();
    let mut trailers = http::HeaderMap::new();
    trailers.insert(
        "grpc-status",
        http::HeaderValue::from_str(&(code as i32).to_string()).unwrap(),
    );
    if let Ok(val) = http::HeaderValue::from_str(&message) {
        trailers.insert("grpc-message", val);
    }

    let response_body = StreamBody::new(futures::stream::iter([
        Ok::<http_body::Frame<Bytes>, Infallible>(http_body::Frame::trailers(trailers)),
    ]))
    .boxed();

    Response::builder()
        .status(200)
        .header("content-type", "application/grpc")
        .body(response_body)
        .unwrap()
}

fn collect_metadata(headers: &http::HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(k, v)| match v.to_str() {
            Ok(vs) => Some((k.to_string(), vs.to_string())),
            Err(_) => {
                tracing::warn!(header = %k, "dropping non-UTF-8 gRPC request header");
                None
            }
        })
        .collect()
}

fn map_error(e: GrpcInboundError) -> (tonic::Code, String) {
    match e {
        GrpcInboundError::Internal(m)         => (tonic::Code::Internal, m),
        GrpcInboundError::NotFound(m)         => (tonic::Code::NotFound, m),
        GrpcInboundError::InvalidArgument(m)  => (tonic::Code::InvalidArgument, m),
        GrpcInboundError::Unavailable(m)      => (tonic::Code::Unavailable, m),
        GrpcInboundError::DeadlineExceeded(m) => (tonic::Code::DeadlineExceeded, m),
        GrpcInboundError::PermissionDenied(m) => (tonic::Code::PermissionDenied, m),
        GrpcInboundError::Unimplemented(m)    => (tonic::Code::Unimplemented, m),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── map_error ─────────────────────────────────────────────────────────

    #[test]
    fn test_map_error_maps_all_grpc_inbound_error_variants() {
        let cases = [
            (GrpcInboundError::Internal("x".into()),         tonic::Code::Internal),
            (GrpcInboundError::NotFound("x".into()),         tonic::Code::NotFound),
            (GrpcInboundError::InvalidArgument("x".into()),  tonic::Code::InvalidArgument),
            (GrpcInboundError::Unavailable("x".into()),      tonic::Code::Unavailable),
            (GrpcInboundError::DeadlineExceeded("x".into()), tonic::Code::DeadlineExceeded),
            (GrpcInboundError::PermissionDenied("x".into()), tonic::Code::PermissionDenied),
            (GrpcInboundError::Unimplemented("x".into()),    tonic::Code::Unimplemented),
        ];
        for (err, expected_code) in cases {
            let (code, _) = map_error(err);
            assert_eq!(code, expected_code);
        }
    }

    // ── grpc_error ────────────────────────────────────────────────────────

    #[test]
    fn test_grpc_error_returns_200_with_grpc_content_type() {
        let resp = grpc_error(tonic::Code::NotFound, "missing");
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/grpc"
        );
    }

    // ── collect_metadata ──────────────────────────────────────────────────

    #[test]
    fn test_collect_metadata_extracts_utf8_header_values() {
        let mut map = http::HeaderMap::new();
        map.insert("x-request-id", "abc-123".parse().unwrap());
        let meta = collect_metadata(&map);
        assert_eq!(meta.get("x-request-id"), Some(&"abc-123".to_string()));
    }
}
