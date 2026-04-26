//! gRPC inbound trait — handles incoming gRPC requests.

use std::pin::Pin;

use futures::future::BoxFuture;

use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse};

/// Result type for gRPC inbound operations.
pub type GrpcInboundResult<T> = Result<T, GrpcInboundError>;

/// A stream of raw gRPC message bytes — one item per decoded gRPC frame.
pub type GrpcMessageStream =
    Pin<Box<dyn futures::Stream<Item = GrpcInboundResult<Vec<u8>>> + Send>>;

/// Error type for gRPC inbound operations.
#[derive(Debug, thiserror::Error)]
pub enum GrpcInboundError {
    #[error("internal: {0}")]
    Internal(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("deadline exceeded: {0}")]
    DeadlineExceeded(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("unimplemented: {0}")]
    Unimplemented(String),
}

/// Minimal health-check result for the gRPC domain.
#[derive(Debug, Clone)]
pub struct GrpcHealthCheck {
    pub healthy: bool,
    pub message: Option<String>,
}

impl GrpcHealthCheck {
    pub fn healthy() -> Self { Self { healthy: true, message: None } }
    pub fn unhealthy(msg: impl Into<String>) -> Self { Self { healthy: false, message: Some(msg.into()) } }
}

/// Handles inbound gRPC requests (server-side).
pub trait GrpcInbound: Send + Sync {
    /// Handle a single unary request.
    fn handle_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>>;

    /// Handle a streaming request (client-streaming, server-streaming, or bidi).
    ///
    /// The default implementation reads the first message from the input stream,
    /// forwards it to [`handle_unary`], and wraps the response in a single-item
    /// output stream.  Implementors that need true streaming override this method.
    ///
    /// [`handle_unary`]: GrpcInbound::handle_unary
    fn handle_stream(
        &self,
        method: String,
        metadata: GrpcMetadata,
        messages: GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcInboundResult<GrpcMessageStream>> {
        Box::pin(async move {
            use futures::StreamExt;
            let mut messages = messages;
            let body = match messages.next().await {
                Some(Ok(b))  => b,
                Some(Err(e)) => return Err(e),
                None         => vec![],
            };
            let req  = GrpcRequest { method, body, metadata };
            let resp = self.handle_unary(req).await?;
            let out: GrpcMessageStream = Box::pin(futures::stream::once(
                futures::future::ready(Ok(resp.body)),
            ));
            Ok(out)
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
