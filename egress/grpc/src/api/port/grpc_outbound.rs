//! gRPC outbound trait — calls remote gRPC services.

use futures::future::BoxFuture;
use thiserror::Error;

use crate::api::value_object::{GrpcRequest, GrpcResponse};

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

/// Makes outbound gRPC calls to remote services.
pub trait GrpcOutbound: Send + Sync {
    fn call_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcOutboundResult<GrpcResponse>>;
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
