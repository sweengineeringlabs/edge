//! gRPC server — binds a socket and delegates all unary calls to a
//! [`GrpcInbound`] handler.  HTTP/2 framing is handled by Hyper directly
//! (avoiding the axum 0.7 / 0.8 type mismatch that Tonic's routing layer
//! would otherwise introduce). gRPC length-prefix framing is handled here.

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use http::{Request, Response};
use http_body_util::{BodyExt, Limited, StreamBody};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::api::interceptor::GrpcInboundInterceptorChain;
use crate::api::port::grpc_inbound::{GrpcInbound, GrpcInboundError, GrpcMessageStream};
use crate::api::value_object::{
    is_reserved_peer_key, CompressionMode, GrpcMetadata, GrpcRequest, GrpcResponse,
    GrpcServerConfig,
};
use crate::core::grpc_timeout::{parse_grpc_timeout, DEFAULT_DEADLINE};
use crate::core::peer_identity::extract_peer_identity;
use crate::core::status_codes::map_inbound_error;
use swe_edge_ingress_tls::{IngressTlsConfig, IngressTlsError};

/// Hard cap on incoming message size.
pub const MAX_MESSAGE_BYTES: usize = 4 * 1_024 * 1_024; // 4 MiB

type BoxBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

/// Error returned by [`TonicGrpcServer::serve`].
#[derive(Debug, thiserror::Error)]
pub enum TonicServerError {
    /// The server could not bind to the requested address.
    #[error("failed to bind to {0}: {1}")]
    Bind(String, #[source] std::io::Error),
    /// TLS configuration could not be loaded or built.
    #[error("TLS: {0}")]
    Tls(#[source] IngressTlsError),
    /// `tls_required` is `true` but no TLS material is configured.
    #[error("server config rejected: {0}")]
    Config(#[source] GrpcServerConfigError),
}

/// Error returned by [`TonicGrpcServer::from_config`] when the supplied
/// server configuration violates a fail-closed invariant.
#[derive(Debug, thiserror::Error)]
pub enum GrpcServerConfigError {
    /// `tls_required` is set but no TLS configuration was attached.
    /// Callers must either supply [`IngressTlsConfig`] or explicitly
    /// call [`GrpcServerConfig::allow_plaintext`].
    #[error(
        "tls_required is set but no TLS configuration supplied — \
         attach an IngressTlsConfig via with_tls(...) or call \
         allow_plaintext() to opt out"
    )]
    TlsRequiredButMissing,
}

/// gRPC server that routes all unary requests through a [`GrpcInbound`] port.
///
/// No proto-gen is required — raw bytes flow directly to the port.
/// Consumers can swap the handler (override) or wrap it in a decorator
/// that also implements [`GrpcInbound`] to add auth, tracing, etc. (extend).
pub struct TonicGrpcServer {
    bind:                   String,
    handler:                Arc<dyn GrpcInbound>,
    max_bytes:              usize,
    max_concurrent_streams: u32,
    tls:                    Option<IngressTlsConfig>,
    interceptors:           GrpcInboundInterceptorChain,
    compression:            CompressionMode,
}

impl TonicGrpcServer {
    /// Create a server that will bind to `bind` and delegate to `handler`.
    pub fn new(bind: impl Into<String>, handler: Arc<dyn GrpcInbound>) -> Self {
        Self {
            bind:                   bind.into(),
            handler,
            max_bytes:              MAX_MESSAGE_BYTES,
            max_concurrent_streams: 100,
            tls:                    None,
            interceptors:           GrpcInboundInterceptorChain::new(),
            compression:            CompressionMode::None,
        }
    }

    /// Construct a server from a [`GrpcServerConfig`].
    ///
    /// **Fail-closed**: if `config.tls_required` is `true` and no
    /// [`IngressTlsConfig`] is supplied (either via `config.tls` or
    /// the optional `tls` arg), returns
    /// [`GrpcServerConfigError::TlsRequiredButMissing`] before any
    /// transport setup.
    pub fn from_config(
        config:  &GrpcServerConfig,
        handler: Arc<dyn GrpcInbound>,
    ) -> Result<Self, GrpcServerConfigError> {
        if config.tls_required && config.tls.is_none() {
            return Err(GrpcServerConfigError::TlsRequiredButMissing);
        }
        Ok(Self {
            bind:                   config.bind.to_string(),
            handler,
            max_bytes:              config.max_message_bytes,
            max_concurrent_streams: config.max_concurrent_streams,
            tls:                    config.tls.clone(),
            interceptors:           GrpcInboundInterceptorChain::new(),
            compression:            config.compression,
        })
    }

    /// Override the maximum incoming message size (default: [`MAX_MESSAGE_BYTES`]).
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_bytes = size;
        self
    }

    /// Override the HTTP/2 SETTINGS_MAX_CONCURRENT_STREAMS advertised
    /// to peers (default: 100).
    pub fn with_max_concurrent_streams(mut self, streams: u32) -> Self {
        self.max_concurrent_streams = streams;
        self
    }

    /// Attach an interceptor chain.  Replaces any previously-set chain.
    /// Interceptors run in registration order — first failure
    /// short-circuits and the handler is never invoked.
    pub fn with_interceptors(mut self, chain: GrpcInboundInterceptorChain) -> Self {
        self.interceptors = chain;
        self
    }

    /// Set the negotiated compression mode.  When `Gzip` or `Zstd`,
    /// the server advertises `grpc-accept-encoding` on responses.
    pub fn with_compression(mut self, mode: CompressionMode) -> Self {
        self.compression = mode;
        self
    }

    /// Enable TLS (or mTLS when [`IngressTlsConfig::client_ca_pem_path`] is
    /// set). The acceptor is built eagerly in [`serve`] / [`serve_with_listener`]
    /// so cert/key errors surface at startup.
    pub fn with_tls(mut self, config: IngressTlsConfig) -> Self {
        self.tls = Some(config);
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
        let bind_addr = listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| self.bind.clone());

        let tls_acceptor = self
            .tls
            .as_ref()
            .map(|cfg| {
                tracing::info!(bind = %bind_addr, mtls = cfg.is_mtls(), "gRPC+TLS server listening");
                swe_edge_ingress_tls::build_tls_acceptor(cfg).map_err(TonicServerError::Tls)
            })
            .transpose()?;

        if tls_acceptor.is_none() {
            tracing::info!(bind = %bind_addr, "gRPC server listening");
        }

        let handler                = self.handler.clone();
        let max_bytes              = self.max_bytes;
        let max_concurrent_streams = self.max_concurrent_streams;
        let interceptors           = self.interceptors.clone();
        let compression            = self.compression;
        let mut shutdown           = std::pin::pin!(shutdown);

        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (stream, _) = match res {
                        Ok(s)  => s,
                        Err(e) => { tracing::warn!("gRPC accept error: {e}"); continue; }
                    };
                    let handler      = handler.clone();
                    let tls_acceptor = tls_acceptor.clone();
                    let interceptors = interceptors.clone();
                    tokio::spawn(async move {
                        if let Some(acceptor) = tls_acceptor {
                            match acceptor.accept(stream).await {
                                Ok(tls) => {
                                    // Snapshot peer identity once per connection
                                    // — every request on this HTTP/2 conn shares
                                    // the same TLS handshake and thus the same
                                    // identity.
                                    let (_, conn_state) = tls.get_ref();
                                    let peer_metadata: HashMap<String, String> = conn_state
                                        .peer_certificates()
                                        .and_then(|chain| chain.first())
                                        .map(|leaf| extract_peer_identity(leaf.as_ref()))
                                        .unwrap_or_default();

                                    let io = TokioIo::new(tls);
                                    let svc = hyper::service::service_fn({
                                        let handler         = handler.clone();
                                        let interceptors    = interceptors.clone();
                                        let peer_metadata   = peer_metadata.clone();
                                        move |req| {
                                            let handler       = handler.clone();
                                            let interceptors  = interceptors.clone();
                                            let peer_metadata = peer_metadata.clone();
                                            async move {
                                                Ok::<_, Infallible>(dispatch(
                                                    req,
                                                    handler,
                                                    max_bytes,
                                                    interceptors,
                                                    compression,
                                                    peer_metadata,
                                                ).await)
                                            }
                                        }
                                    });
                                    if let Err(e) = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
                                        .max_concurrent_streams(max_concurrent_streams)
                                        .serve_connection(io, svc)
                                        .await
                                    {
                                        tracing::debug!("gRPC+TLS connection error: {e}");
                                    }
                                }
                                Err(e) => tracing::debug!("gRPC TLS handshake failed: {e}"),
                            }
                        } else {
                            // Plaintext connection — no peer identity available.
                            let io = TokioIo::new(stream);
                            let svc = hyper::service::service_fn({
                                let handler      = handler.clone();
                                let interceptors = interceptors.clone();
                                move |req| {
                                    let handler      = handler.clone();
                                    let interceptors = interceptors.clone();
                                    async move {
                                        Ok::<_, Infallible>(dispatch(
                                            req,
                                            handler,
                                            max_bytes,
                                            interceptors,
                                            compression,
                                            HashMap::new(),
                                        ).await)
                                    }
                                }
                            });
                            if let Err(e) = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
                                .max_concurrent_streams(max_concurrent_streams)
                                .serve_connection(io, svc)
                                .await
                            {
                                tracing::debug!("gRPC connection error: {e}");
                            }
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

/// Read the per-call deadline from the `grpc-timeout` header, falling back
/// to [`DEFAULT_DEADLINE`] when the header is absent or malformed.
fn read_deadline(headers: &http::HeaderMap) -> Duration {
    headers
        .get("grpc-timeout")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_grpc_timeout)
        .unwrap_or(DEFAULT_DEADLINE)
}

async fn dispatch(
    req:           Request<hyper::body::Incoming>,
    handler:       Arc<dyn GrpcInbound>,
    max_bytes:     usize,
    interceptors:  GrpcInboundInterceptorChain,
    compression:   CompressionMode,
    peer_metadata: HashMap<String, String>,
) -> Response<BoxBody> {
    let method   = req.uri().path().to_string();
    let metadata = collect_metadata(req.headers());
    let deadline = read_deadline(req.headers());

    // Past-deadline calls MUST fail before the handler runs.  A zero
    // deadline (e.g. `grpc-timeout: 0n` or `0S`) means the client gave
    // us no time at all to do work.
    if deadline.is_zero() {
        return grpc_error(
            tonic::Code::DeadlineExceeded,
            "request rejected before handler dispatch: deadline has already elapsed",
        );
    }

    let body_bytes = match Limited::new(req.into_body(), max_bytes).collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_)        => return grpc_error(tonic::Code::ResourceExhausted, "message too large"),
    };

    // Per-request cancellation token — fired implicitly when this future is
    // dropped (i.e. the client closed the HTTP/2 stream).  The handler
    // observes the token via the `messages` stream's parent scope; we expose
    // it on the bridge below.
    let cancel = CancellationToken::new();
    let _drop_guard = cancel.clone().drop_guard();

    // Decode all gRPC length-prefix frames from the body.
    let frames = decode_grpc_frames(&body_bytes);
    let message_stream: GrpcMessageStream = Box::pin(futures::stream::iter(
        frames.into_iter().map(|f| Ok::<Vec<u8>, GrpcInboundError>(f.to_vec())),
    ));

    // Build merged request metadata.  Reserved peer-identity keys
    // supplied by the client over the wire are stripped first — the
    // server is the only party allowed to set them, and only after a
    // successful mTLS handshake.  Then we inject the cryptographically
    // derived peer identity (empty for plaintext / TLS-only conns).
    let mut headers: HashMap<String, String> = metadata
        .iter()
        .filter(|(k, _)| !is_reserved_peer_key(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (k, v) in peer_metadata.iter() {
        headers.insert(k.clone(), v.clone());
    }
    headers.insert(
        "x-edge-grpc-deadline-millis".to_string(),
        deadline.as_millis().to_string(),
    );

    // Build a synthetic GrpcRequest envelope so interceptors can
    // observe headers + body before dispatch.
    let mut intercept_req = GrpcRequest::new(
        method.clone(),
        body_bytes.to_vec(),
        deadline,
    );
    intercept_req.metadata = GrpcMetadata { headers: headers.clone() };

    // Run before_dispatch — first failure short-circuits and the
    // handler never runs.
    if let Err(e) = interceptors.run_before(&mut intercept_req) {
        let (code, msg) = map_inbound_error(e);
        return grpc_error(code, msg);
    }
    // Interceptors may have mutated headers; pull them back.
    let merged_headers = intercept_req.metadata.headers.clone();

    // Race the handler future against the deadline — past-deadline
    // mid-handler is a server-side `DeadlineExceeded` and must NOT
    // propagate handler partial output.
    let handler_fut = handler.handle_stream(
        method,
        GrpcMetadata { headers: merged_headers },
        message_stream,
    );
    let cancel_fut  = cancel.cancelled();

    let result = tokio::select! {
        biased;
        // Cancellation: client disconnected — abort and never produce a body.
        _ = cancel_fut => {
            return grpc_error(tonic::Code::Cancelled, "client disconnected");
        }
        // Deadline: timer fired before the handler returned.
        _ = tokio::time::sleep(deadline) => {
            return grpc_error(
                tonic::Code::DeadlineExceeded,
                "handler deadline exceeded",
            );
        }
        r = handler_fut => r,
    };

    match result {
        Ok((resp_stream, resp_meta)) => {
            // Drain the handler stream so interceptors can observe the
            // response payload + metadata before it goes out on the wire.
            //
            // Buffered by design: `after_dispatch` operates on a single
            // body bag, not a stream — true streaming interceptors are
            // a follow-up.
            let collected_payload = match collect_response_payload(resp_stream).await {
                Ok(p)  => p,
                Err(e) => {
                    let (code, msg) = map_inbound_error(e);
                    return grpc_error(code, msg);
                }
            };
            // Synthesise an interceptor-visible response.  The body
            // surface is the concatenation of all stream frames — when
            // an after_dispatch hook mutates it we send the mutated
            // bytes as a single frame; otherwise we preserve the
            // original frame boundaries.
            let original_payload = collected_payload.clone();
            let mut response = GrpcResponse {
                body:     collected_payload.concat(),
                metadata: resp_meta,
            };

            // Advertise grpc-accept-encoding when compression is enabled.
            if let Some(name) = compression.header_value() {
                response
                    .metadata
                    .headers
                    .entry("grpc-accept-encoding".to_string())
                    .or_insert_with(|| name.to_string());
            }

            // after_dispatch — same short-circuit semantics as before.
            if let Err(e) = interceptors.run_after(&mut response) {
                let (code, msg) = map_inbound_error(e);
                return grpc_error(code, msg);
            }

            let body_changed = response.body != original_payload.concat();
            let payloads = if body_changed {
                vec![response.body]
            } else {
                original_payload
            };

            grpc_stream_response_from_payloads(payloads, response.metadata).await
        }
        Err(e) => {
            let (code, msg) = map_inbound_error(e);
            grpc_error(code, msg)
        }
    }
}

/// Drain a [`GrpcMessageStream`] into a list of payloads.
async fn collect_response_payload(
    mut stream: GrpcMessageStream,
) -> Result<Vec<Vec<u8>>, GrpcInboundError> {
    use futures::StreamExt;
    let mut out = Vec::new();
    while let Some(item) = stream.next().await {
        out.push(item?);
    }
    Ok(out)
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

/// Build an HTTP/2 response from already-buffered payloads + final metadata.
async fn grpc_stream_response_from_payloads(
    payloads: Vec<Vec<u8>>,
    meta:     GrpcMetadata,
) -> Response<BoxBody> {
    let mut frames: Vec<Bytes> = Vec::with_capacity(payloads.len());
    for payload in payloads {
        let mut buf = BytesMut::with_capacity(5 + payload.len());
        buf.put_u8(0); // not compressed
        buf.put_u32(payload.len() as u32);
        buf.put_slice(&payload);
        frames.push(buf.freeze());
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

/// Collect a response stream into a single HTTP/2 response with one DATA frame
/// per stream item plus a trailing `grpc-status=0` header and any response metadata.
#[allow(dead_code)]
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
                let (code, msg) = map_inbound_error(e);
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

// `map_error` was replaced by `map_inbound_error` in
// `crate::core::status_codes` — kept here as a cross-reference comment.

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── read_deadline ─────────────────────────────────────────────────────

    /// @covers: read_deadline — falls back to DEFAULT_DEADLINE when header absent.
    #[test]
    fn test_read_deadline_falls_back_to_default_when_header_absent() {
        let map = http::HeaderMap::new();
        assert_eq!(read_deadline(&map), DEFAULT_DEADLINE);
    }

    /// @covers: read_deadline — parses well-formed grpc-timeout header.
    #[test]
    fn test_read_deadline_parses_grpc_timeout_header() {
        let mut map = http::HeaderMap::new();
        map.insert("grpc-timeout", "500m".parse().unwrap());
        assert_eq!(read_deadline(&map), Duration::from_millis(500));
    }

    /// @covers: read_deadline — malformed header falls back to default rather than panicking.
    #[test]
    fn test_read_deadline_falls_back_on_malformed_header() {
        let mut map = http::HeaderMap::new();
        map.insert("grpc-timeout", "garbage".parse().unwrap());
        assert_eq!(read_deadline(&map), DEFAULT_DEADLINE);
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
