//! gRPC inbound trait — handles incoming gRPC requests.

use futures::future::BoxFuture;

use crate::api::grpc::{GrpcRequest, GrpcResponse};
use crate::api::health_check::HealthCheck;
use crate::api::ingress_error::IngressResult;

/// Handles inbound gRPC requests (server-side).
pub trait GrpcInbound: Send + Sync {
    fn handle_unary(&self, request: GrpcRequest) -> BoxFuture<'_, IngressResult<GrpcResponse>>;
    fn health_check(&self) -> BoxFuture<'_, IngressResult<HealthCheck>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_inbound_is_object_safe() {
        fn _assert_object_safe(_: &dyn GrpcInbound) {}
    }
}
