//! gRPC outbound trait — calls remote gRPC services.

use futures::future::BoxFuture;

use crate::api::egress_error::EgressResult;
use crate::api::grpc::{GrpcRequest, GrpcResponse};
use crate::api::health_check::HealthCheck;

/// Makes outbound gRPC calls to remote services.
pub trait GrpcOutbound: Send + Sync {
    fn call_unary(&self, request: GrpcRequest) -> BoxFuture<'_, EgressResult<GrpcResponse>>;
    fn health_check(&self) -> BoxFuture<'_, EgressResult<HealthCheck>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_outbound_is_object_safe() {
        fn _assert_object_safe(_: &dyn GrpcOutbound) {}
    }
}
