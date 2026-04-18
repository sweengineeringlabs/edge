//! Default core implementation of the gRPC gateway contract.
//!
//! The [`NoopGrpcGateway`] impl provides a no-op baseline that satisfies
//! rule 49 ("core modules must implement their api traits"). It reports
//! healthy and returns `NotSupported` for any actual RPC invocation.
//!
//! Production consumers typically supply their own [`GrpcGateway`]
//! implementations (e.g. tonic-backed clients/servers) and wire them in
//! via `saf/` factories. This baseline exists so the crate is always
//! compile-checkable end to end and so the trait has a concrete
//! reference implementation living next to it in core/.

use futures::future::BoxFuture;

use crate::api::grpc::{GrpcRequest, GrpcResponse};
use crate::api::traits::{GrpcGateway, GrpcInbound, GrpcOutbound};
use crate::api::types::{GatewayError, GatewayResult, HealthCheck};

/// A no-op gRPC gateway used as the default core implementation.
///
/// All RPC calls return `GatewayError::NotSupported`. The health check
/// reports [`HealthCheck::healthy`] so the component can participate in
/// aggregate readiness reporting without masking real backend failures.
pub(crate) struct NoopGrpcGateway;

impl NoopGrpcGateway {
    /// Create a new no-op gRPC gateway.
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self
    }
}

impl GrpcInbound for NoopGrpcGateway {
    fn handle_unary(
        &self,
        _request: GrpcRequest,
    ) -> BoxFuture<'_, GatewayResult<GrpcResponse>> {
        Box::pin(async {
            Err(GatewayError::NotSupported(
                "NoopGrpcGateway does not handle unary RPCs".into(),
            ))
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GatewayResult<HealthCheck>> {
        Box::pin(async { Ok(HealthCheck::healthy()) })
    }
}

impl GrpcOutbound for NoopGrpcGateway {
    fn call_unary(
        &self,
        _endpoint: &str,
        _request: GrpcRequest,
    ) -> BoxFuture<'_, GatewayResult<GrpcResponse>> {
        Box::pin(async {
            Err(GatewayError::NotSupported(
                "NoopGrpcGateway does not call unary RPCs".into(),
            ))
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GatewayResult<HealthCheck>> {
        Box::pin(async { Ok(HealthCheck::healthy()) })
    }
}

impl GrpcGateway for NoopGrpcGateway {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::grpc::GrpcMetadata;

    fn sample_request() -> GrpcRequest {
        GrpcRequest {
            method: "svc.v1/Method".into(),
            body: vec![],
            metadata: GrpcMetadata::default(),
        }
    }

    /// @covers: NoopGrpcGateway::handle_unary rejects with NotSupported
    #[tokio::test]
    async fn test_handle_unary_rejects_with_not_supported() {
        let gw = NoopGrpcGateway::new();
        let err = GrpcInbound::handle_unary(&gw, sample_request())
            .await
            .expect_err("noop gateway must reject handle_unary");
        assert!(
            matches!(err, GatewayError::NotSupported(_)),
            "expected NotSupported, got {err:?}"
        );
    }

    /// @covers: NoopGrpcGateway::call_unary rejects with NotSupported
    #[tokio::test]
    async fn test_call_unary_rejects_with_not_supported() {
        let gw = NoopGrpcGateway::new();
        let err = GrpcOutbound::call_unary(&gw, "endpoint", sample_request())
            .await
            .expect_err("noop gateway must reject call_unary");
        assert!(
            matches!(err, GatewayError::NotSupported(_)),
            "expected NotSupported, got {err:?}"
        );
    }

    /// @covers: NoopGrpcGateway reports healthy on health_check
    #[tokio::test]
    async fn test_health_check_reports_healthy() {
        let gw = NoopGrpcGateway::new();
        let status = GrpcInbound::health_check(&gw).await.unwrap();
        assert!(matches!(status.status, crate::api::types::HealthStatus::Healthy));
    }

    /// @covers: NoopGrpcGateway can be used as `dyn GrpcGateway`
    #[test]
    fn test_is_usable_as_grpc_gateway_trait_object() {
        fn _accepts(_g: &dyn GrpcGateway) {}
        let gw = NoopGrpcGateway::new();
        _accepts(&gw);
    }
}
