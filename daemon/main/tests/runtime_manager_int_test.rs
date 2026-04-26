//! Integration tests for the swe_edge_daemon SAF runtime_manager surface.

use std::sync::Arc;

use futures::future::BoxFuture;
use swe_edge_daemon::{
    EgressGateway, IngressGateway, RuntimeConfig, RuntimeManager, RuntimeStatus,
    runtime_manager,
};
use swe_edge_ingress::{
    HttpHealthCheck, HttpInbound, HttpInboundResult, HttpRequest, HttpResponse,
};
use swe_edge_egress::{
    HttpOutbound, HttpOutboundResult, HttpRequest as EgressReq, HttpResponse as EgressResp,
};
use edge_controller::{HealthReport, LifecycleError, LifecycleMonitor};
use async_trait::async_trait;

struct StubLifecycle;

#[async_trait]
impl LifecycleMonitor for StubLifecycle {
    async fn health(&self) -> HealthReport { HealthReport::from_components(vec![]) }
    async fn start_background_tasks(&self) {}
    async fn shutdown(&self) -> Result<(), LifecycleError> { Ok(()) }
}

struct StubHttpInbound;
impl HttpInbound for StubHttpInbound {
    fn handle(&self, _: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async { Ok(HttpResponse::new(200, vec![])) })
    }
    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

struct StubHttpOutbound;
impl HttpOutbound for StubHttpOutbound {
    fn send(&self, _: EgressReq) -> BoxFuture<'_, HttpOutboundResult<EgressResp>> {
        Box::pin(async { Ok(EgressResp::new(200, vec![])) })
    }
    fn health_check(&self) -> BoxFuture<'_, HttpOutboundResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

fn make_runtime() -> impl RuntimeManager {
    runtime_manager(
        RuntimeConfig::default().with_systemd_notify(false),
        IngressGateway::http(Arc::new(StubHttpInbound)),
        EgressGateway::http(Arc::new(StubHttpOutbound)),
        Arc::new(StubLifecycle),
    )
}

/// @covers: runtime_manager
#[tokio::test]
async fn test_runtime_manager_start_and_shutdown_round_trip() {
    let m = make_runtime();
    m.start().await.expect("start ok");
    let h = m.health().await;
    assert!(h.status == RuntimeStatus::Running);
    m.shutdown().await.expect("shutdown ok");
}

/// @covers: runtime_manager
#[tokio::test]
async fn test_runtime_manager_health_reports_ingress_and_egress() {
    let m = make_runtime();
    m.start().await.expect("start ok");
    let h = m.health().await;
    let names: Vec<&str> = h.components.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"ingress.http"));
    assert!(names.contains(&"egress.http"));
}
