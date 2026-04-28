//! gRPC inbound trait — handles incoming gRPC requests.

use std::pin::Pin;

use futures::future::BoxFuture;

use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};

/// Result type for gRPC inbound operations.
pub type GrpcInboundResult<T> = Result<T, GrpcInboundError>;

/// A stream of raw gRPC message bytes — one item per decoded gRPC frame.
pub type GrpcMessageStream =
    Pin<Box<dyn futures::Stream<Item = GrpcInboundResult<Vec<u8>>> + Send>>;

/// Error type for gRPC inbound operations.
///
/// `Status(GrpcStatusCode, String)` is the canonical generic variant for any
/// gRPC error condition — handlers SHOULD prefer it over the named variants
/// when they need a status code that is not Internal/NotFound/etc.  Both
/// representations are recognised by [`crate::core::status_codes::map_inbound_error`].
///
/// The named variants (`Internal`, `NotFound`, `InvalidArgument`,
/// `Unavailable`, `DeadlineExceeded`, `PermissionDenied`, `Unimplemented`)
/// are kept for ergonomic call sites and for backwards source compatibility.
///
/// ## Hygiene contract
///
/// The string carried by `Internal(_)` is treated as a *server-side log
/// message* and may contain stack-traces or struct names.  The dispatch
/// layer logs it at WARN with `tracing::warn!`, then surfaces only a
/// fixed sanitized string on the wire.  Other variants pass their
/// message through verbatim — they are expected to be already
/// caller-safe (e.g. "no such row", "invalid argument 'foo'").
#[derive(Debug, thiserror::Error)]
pub enum GrpcInboundError {
    /// A gRPC status code with a sanitized message.  Preferred for new code.
    #[error("status {0:?}: {1}")]
    Status(GrpcStatusCode, String),
    /// Internal server error.  String is logged but NEVER sent on the wire.
    #[error("internal: {0}")]
    Internal(String),
    /// Resource not found.
    #[error("not found: {0}")]
    NotFound(String),
    /// Request argument failed validation.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// Service unavailable.
    #[error("unavailable: {0}")]
    Unavailable(String),
    /// Request deadline exceeded.
    #[error("deadline exceeded: {0}")]
    DeadlineExceeded(String),
    /// Caller lacks permission.
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    /// Method not implemented.
    #[error("unimplemented: {0}")]
    Unimplemented(String),
}

/// Minimal health-check result for the gRPC domain.
#[derive(Debug, Clone)]
pub struct GrpcHealthCheck {
    /// `true` when the handler is healthy.
    pub healthy: bool,
    /// Optional human-readable status detail.
    pub message: Option<String>,
}

impl GrpcHealthCheck {
    /// Create a healthy result.
    pub fn healthy() -> Self { Self { healthy: true, message: None } }
    /// Create an unhealthy result with a message.
    pub fn unhealthy(msg: impl Into<String>) -> Self { Self { healthy: false, message: Some(msg.into()) } }
}

/// Handles inbound gRPC requests (server-side).
pub trait GrpcInbound: Send + Sync {
    /// Handle a single unary request.
    fn handle_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>>;

    /// Handle a streaming request (client-streaming, server-streaming, or bidi).
    ///
    /// Returns the response stream **and** any response metadata (trailers) as a
    /// tuple `(stream, metadata)`. The server threads the metadata into HTTP/2
    /// trailers alongside `grpc-status: 0`.
    ///
    /// The default implementation reads the first message from the input stream,
    /// forwards it to [`handle_unary`], and wraps the response in a single-item
    /// output stream, preserving the response metadata.  Implementors that need
    /// true streaming override this method.
    ///
    /// [`handle_unary`]: GrpcInbound::handle_unary
    fn handle_stream(
        &self,
        method: String,
        metadata: GrpcMetadata,
        messages: GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcInboundResult<(GrpcMessageStream, GrpcMetadata)>> {
        Box::pin(async move {
            use futures::StreamExt;
            let mut messages = messages;
            let body = match messages.next().await {
                Some(Ok(b))  => b,
                Some(Err(e)) => return Err(e),
                None         => vec![],
            };
            // Synthesised request: deadline carried by the caller of handle_stream
            // is preserved in metadata via grpc-timeout already; default here is
            // the server-default that handle_stream had access to.  Long term the
            // signature should accept a deadline; for now use a generous default.
            let req  = GrpcRequest::new(method, body, std::time::Duration::from_secs(30))
                .with_metadata(metadata);
            let resp = self.handle_unary(req).await?;
            let out: GrpcMessageStream = Box::pin(futures::stream::once(
                futures::future::ready(Ok(resp.body)),
            ));
            Ok((out, resp.metadata))
        })
    }

    /// Perform a health check.
    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_inbound_is_object_safe() {
        fn _assert_object_safe(_: &dyn GrpcInbound) {}
    }

    #[test]
    fn test_grpc_inbound_error_internal_formats_correctly() {
        let err = GrpcInboundError::Internal("fail".into());
        assert!(err.to_string().contains("fail"));
    }

    /// @covers: GrpcInboundError::Status — generic variant carries enum + msg.
    #[test]
    fn test_grpc_inbound_error_status_variant_carries_code_and_message() {
        let err = GrpcInboundError::Status(GrpcStatusCode::Aborted, "tx aborted".into());
        let s   = err.to_string();
        assert!(s.contains("Aborted"));
        assert!(s.contains("tx aborted"));
    }

    #[test]
    fn test_grpc_health_check_healthy_is_true() {
        let h = GrpcHealthCheck::healthy();
        assert!(h.healthy);
    }

    #[test]
    fn test_grpc_health_check_unhealthy_sets_message() {
        let h = GrpcHealthCheck::unhealthy("down");
        assert!(!h.healthy);
        assert_eq!(h.message.as_deref(), Some("down"));
    }
}
