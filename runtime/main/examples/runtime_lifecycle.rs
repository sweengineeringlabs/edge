//! Runtime lifecycle — config loading, gateway assembly, start, health, shutdown.
//!
//! Run:
//!     cargo run -p swe-edge-runtime --example runtime_lifecycle
//!
//! Demonstrates:
//!   1. RuntimeConfig loading and customisation via the builder API
//!   2. DefaultInput + DefaultOutput assembly from SAF constructors
//!   3. RuntimeManager lifecycle: start → health → shutdown
//!
//! SEA constraint: public API is accessed only through each crate's SAF surface.
//! The stub inbound/outbound adapters are defined locally — they stand in for
//! the real axum/tonic/reqwest implementations a consumer app would supply.

use std::sync::Arc;

use futures::future::BoxFuture;
use swe_edge_egress::{
    HttpOutbound, HttpOutboundResult,
    HttpRequest as EgressReq, HttpResponse as EgressResp,
};
use swe_edge_ingress::{
    HttpHealthCheck, HttpInbound, HttpInboundResult, HttpRequest, HttpResponse,
};
use swe_edge_runtime::{
    runtime_manager, DefaultInput, DefaultOutput, RuntimeConfig, RuntimeManager, RuntimeStatus,
};
use edge_proxy::new_null_lifecycle_monitor;

// ── stub inbound ──────────────────────────────────────────────────────────────

struct NoopInbound;

impl HttpInbound for NoopInbound {
    fn handle(&self, _: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async { Ok(HttpResponse::new(204, vec![])) })
    }

    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

// ── stub outbound ─────────────────────────────────────────────────────────────

struct NoopOutbound;

impl HttpOutbound for NoopOutbound {
    fn send(&self, _: EgressReq) -> BoxFuture<'_, HttpOutboundResult<EgressResp>> {
        Box::pin(async { Ok(EgressResp::new(200, vec![])) })
    }

    fn health_check(&self) -> BoxFuture<'_, HttpOutboundResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Config — customise the defaults via the builder API.
    let config = RuntimeConfig::default()
        .with_service_name("example-service")
        .with_http_bind("127.0.0.1:8080")
        .with_shutdown_timeout(5);
    println!("service:  {}", config.service_name);
    println!("http:     {}", config.http_bind);
    println!("timeout:  {}s", config.shutdown_timeout_secs);

    // 2. Assemble gateways — DefaultInput and DefaultOutput accept `Arc<dyn Trait>`
    //    so the runtime never names or imports concrete adapter types.
    let ingress   = Arc::new(DefaultInput::new_http(Arc::new(NoopInbound)));
    let egress    = Arc::new(DefaultOutput::new_http(Arc::new(NoopOutbound)));
    let lifecycle = new_null_lifecycle_monitor();

    // 3. Build the RuntimeManager via the SAF factory (returns `impl RuntimeManager`).
    let mgr = runtime_manager(config, ingress, egress, lifecycle);

    // 4. Start — probes each configured transport and transitions status to Running.
    mgr.start().await?;
    let health = mgr.health().await;
    println!("\nafter start:");
    println!("  status: {:?}", health.status);
    assert_eq!(health.status, RuntimeStatus::Running);
    for c in &health.components {
        println!("  component {} — healthy={}", c.name, c.healthy);
    }

    // 5. Shutdown — drains in-flight work and transitions status to Stopped.
    mgr.shutdown().await?;
    let health = mgr.health().await;
    println!("\nafter shutdown:");
    println!("  status: {:?}", health.status);
    assert_eq!(health.status, RuntimeStatus::Stopped);

    Ok(())
}
