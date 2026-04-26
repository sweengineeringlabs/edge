//! gRPC outbound trait — calls remote gRPC services.

use futures::future::BoxFuture;
use thiserror::Error;

use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse};

/// Error type for gRPC outbound operations.
#[derive(Debug, Error)]
pub enum GrpcOutboundError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("internal: {0}")]
    Internal(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
}

/// Result type for gRPC outbound operations.
pub type GrpcOutboundResult<T> = Result<T, GrpcOutboundError>;

/// A stream of gRPC message payloads (each item is a raw decoded frame body).
pub type GrpcMessageStream =
    std::pin::Pin<Box<dyn futures::Stream<Item = GrpcOutboundResult<Vec<u8>>> + Send>>;

/// Makes outbound gRPC calls to remote services.
pub trait GrpcOutbound: Send + Sync {
    /// Send a single unary gRPC request and receive a single response.
    fn call_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcOutboundResult<GrpcResponse>>;

    /// Send a streaming gRPC request and receive a response stream.
    ///
    /// The default implementation returns `Unimplemented` — override to enable streaming.
    fn call_stream(
        &self,
        method: String,
        metadata: GrpcMetadata,
        messages: GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcOutboundResult<GrpcMessageStream>> {
        let _ = (method, metadata, messages);
        Box::pin(futures::future::ready(Err(GrpcOutboundError::Internal(
            "streaming not supported".into(),
        ))))
    }

    /// Check that the remote endpoint is reachable.
    fn health_check(&self) -> BoxFuture<'_, GrpcOutboundResult<()>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_outbound_is_object_safe() {
        fn _assert_object_safe(_: &dyn GrpcOutbound) {}
    }
}
