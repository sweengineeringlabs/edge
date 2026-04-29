//! gRPC outbound trait — calls remote gRPC services.

use futures::future::BoxFuture;
use thiserror::Error;

use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};

/// Error type for gRPC outbound operations.
///
/// `Status(GrpcStatusCode, String)` is the canonical variant for a
/// well-formed gRPC reply that carried a non-`Ok` status code.  The
/// string is a *sanitized* message suitable to surface to callers and
/// must not contain server-side stack traces or internal paths.  See
/// `core::status_codes` for the wire ↔ enum mapping.
///
/// `ConnectionFailed`, `Timeout`, `Unavailable`, `Cancelled`, and
/// `Internal` are transport-level conditions that occur *before*
/// the server returns a status (or after the local timeout fires).
#[derive(Debug, Error)]
pub enum GrpcOutboundError {
    /// The remote returned a non-`Ok` gRPC status with a sanitized message.
    #[error("status {0:?}: {1}")]
    Status(GrpcStatusCode, String),
    /// The transport could not establish a connection.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    /// The local per-call deadline elapsed before a response was received.
    #[error("timeout: {0}")]
    Timeout(String),
    /// An unexpected client-side condition.
    #[error("internal: {0}")]
    Internal(String),
    /// The remote endpoint was unavailable.
    #[error("unavailable: {0}")]
    Unavailable(String),
    /// The caller cancelled the in-flight request via the supplied token.
    #[error("cancelled: {0}")]
    Cancelled(String),
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
        Box::pin(futures::future::ready(Err(GrpcOutboundError::Status(
            GrpcStatusCode::Unimplemented,
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

    /// Verifies that the default `call_stream` impl returns `Status(Unimplemented, _)`
    /// without requiring an override — any `GrpcOutbound` impl gets this for free
    /// and it must not silently succeed.
    #[tokio::test]
    async fn test_call_stream_default_returns_unimplemented_status() {
        use std::time::Duration;
        use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse};

        struct UnaryOnlyClient;
        impl GrpcOutbound for UnaryOnlyClient {
            fn call_unary(
                &self,
                _: GrpcRequest,
            ) -> futures::future::BoxFuture<'_, GrpcOutboundResult<GrpcResponse>> {
                Box::pin(futures::future::ready(Ok(GrpcResponse {
                    body:     vec![],
                    metadata: GrpcMetadata::default(),
                })))
            }
            fn health_check(&self) -> futures::future::BoxFuture<'_, GrpcOutboundResult<()>> {
                Box::pin(futures::future::ready(Ok(())))
            }
        }

        // Avoid unused-warning on Duration; constructor still requires deadline elsewhere.
        let _ = Duration::from_millis(1);

        let client = UnaryOnlyClient;
        let messages: GrpcMessageStream =
            Box::pin(futures::stream::empty::<GrpcOutboundResult<Vec<u8>>>());
        let result = client
            .call_stream("svc/Method".into(), GrpcMetadata::default(), messages)
            .await;
        let err = match result {
            Err(e) => e,
            Ok(_)  => panic!("default call_stream must return Err, got Ok"),
        };
        match err {
            GrpcOutboundError::Status(GrpcStatusCode::Unimplemented, msg) => {
                assert!(
                    msg.contains("streaming not supported"),
                    "error message was: {msg}"
                );
            }
            other => panic!("expected Status(Unimplemented, _), got {other:?}"),
        }
    }

    /// @covers: GrpcOutboundError — Status variant carries enum + sanitized message.
    #[test]
    fn test_status_variant_carries_code_and_message() {
        let err = GrpcOutboundError::Status(GrpcStatusCode::NotFound, "no such row".into());
        let s   = err.to_string();
        assert!(s.contains("NotFound"));
        assert!(s.contains("no such row"));
    }

    /// @covers: GrpcOutboundError — Cancelled variant exists for caller-cancelled calls.
    #[test]
    fn test_cancelled_variant_renders_with_reason() {
        let err = GrpcOutboundError::Cancelled("token fired".into());
        assert!(err.to_string().contains("token fired"));
    }
}
