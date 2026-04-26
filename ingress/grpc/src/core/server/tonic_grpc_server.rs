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

use crate::api::port::grpc_inbound::{GrpcInbound, GrpcInboundError};
use crate::api::value_object::{GrpcMetadata, GrpcRequest};

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

    // Strip the 5-byte gRPC length-prefix frame (1 compressed flag + 4 length bytes).
    let msg = if body_bytes.len() >= 5 {
        body_bytes.slice(5..)
    } else {
        body_bytes
    };

    let grpc_req = GrpcRequest {
        method,
        body: msg.to_vec(),
        metadata: GrpcMetadata { headers: metadata },
    };

    match handler.handle_unary(grpc_req).await {
        Ok(resp) => grpc_success(resp.body, resp.metadata.headers),
        Err(e)   => {
            let (code, msg) = map_error(e);
            grpc_error(code, msg)
        }
    }
}

// ── gRPC framing helpers ──────────────────────────────────────────────────────

fn grpc_success(body: Vec<u8>, extra_headers: HashMap<String, String>) -> Response<BoxBody> {
    // Build length-prefixed gRPC data frame.
    let mut buf = BytesMut::with_capacity(5 + body.len());
    buf.put_u8(0); // not compressed
    buf.put_u32(body.len() as u32);
    buf.put_slice(&body);

    // Trailing headers carry the gRPC status.
    let mut trailers = http::HeaderMap::new();
    trailers.insert("grpc-status", http::HeaderValue::from_static("0"));
    for (k, v) in &extra_headers {
        if let (Ok(name), Ok(val)) = (
            http::HeaderName::from_bytes(k.as_bytes()),
            http::HeaderValue::from_str(v),
        ) {
            trailers.insert(name, val);
        }
    }

    let response_body = StreamBody::new(futures::stream::iter([
        Ok::<http_body::Frame<Bytes>, Infallible>(http_body::Frame::data(buf.freeze())),
        Ok(http_body::Frame::trailers(trailers)),
    ]))
    .boxed();

    Response::builder()
        .status(200)
        .header("content-type", "application/grpc")
        .body(response_body)
        .unwrap()
}

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
        .filter_map(|(k, v)| v.to_str().ok().map(|vs| (k.to_string(), vs.to_string())))
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

    // ── grpc_success ──────────────────────────────────────────────────────

    #[test]
    fn test_grpc_success_returns_200_with_grpc_content_type() {
        let resp = grpc_success(vec![1, 2, 3], HashMap::new());
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/grpc"
        );
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
