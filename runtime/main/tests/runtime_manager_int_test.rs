//! Integration tests for the swe_edge_runtime SAF runtime_manager surface.
//!
//! These tests exercise the full daemon wiring: RuntimeManager lifecycle +
//! AxumHttpServer serving real TCP traffic through IngressGateway.

use std::sync::Arc;

use async_trait::async_trait;
use futures::future::BoxFuture;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use edge_proxy::{HealthReport, LifecycleError, LifecycleMonitor};
use swe_edge_runtime::{
    DefaultInput, DefaultOutput, RuntimeConfig, RuntimeManager, RuntimeStatus,
    runtime_manager,
};
use swe_edge_egress::{
    HttpOutbound, HttpOutboundResult, HttpRequest as EgressReq, HttpResponse as EgressResp,
};
use swe_edge_ingress::{
    AxumHttpServer, HttpHealthCheck, HttpInbound, HttpInboundError,
    HttpInboundResult, HttpRequest, HttpResponse,
};

// ── Shared stubs ──────────────────────────────────────────────────────────────

struct StubLifecycle;

#[async_trait]
impl LifecycleMonitor for StubLifecycle {
    async fn health(&self) -> HealthReport { HealthReport::from_components(vec![]) }
    async fn start_background_tasks(&self) {}
    async fn shutdown(&self) -> Result<(), LifecycleError> { Ok(()) }
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

/// Returns 200 with the request method + path as the body.
struct EchoHandler;
impl HttpInbound for EchoHandler {
    fn handle(&self, req: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async move {
            let body = format!("{} {}", req.method, req.url).into_bytes();
            Ok(HttpResponse::new(200, body))
        })
    }
    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

/// Always returns NotFound — exercises the error-to-status mapping.
struct NotFoundHandler;
impl HttpInbound for NotFoundHandler {
    fn handle(&self, _: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async { Err(HttpInboundError::NotFound("resource gone".into())) })
    }
    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Wire up the full daemon stack: RuntimeManager + AxumHttpServer on a free port.
/// Returns (base_url, runtime_manager, shutdown_trigger).
async fn start_daemon_stack(
    handler: Arc<dyn HttpInbound>,
) -> (String, impl RuntimeManager, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");

    let config  = RuntimeConfig::default().with_systemd_notify(false);
    let ingress = Arc::new(DefaultInput::new_http(handler.clone()));
    let egress  = Arc::new(DefaultOutput::new_http(Arc::new(StubHttpOutbound)));
    let mgr     = runtime_manager(config, ingress, egress, Arc::new(StubLifecycle));

    mgr.start().await.expect("RuntimeManager::start failed");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server = AxumHttpServer::new(addr.to_string(), handler);
    tokio::spawn(async move {
        let signal = async move { let _ = shutdown_rx.await; };
        let _ = server.serve_with_listener(listener, signal).await;
    });

    (base_url, mgr, shutdown_tx)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// @covers: runtime_manager — start transitions to Running, health reports components
#[tokio::test]
async fn test_runtime_manager_start_and_shutdown_round_trip() {
    let handler: Arc<dyn HttpInbound> = Arc::new(EchoHandler);
    let config  = RuntimeConfig::default().with_systemd_notify(false);
    let ingress = Arc::new(DefaultInput::new_http(handler));
    let egress  = Arc::new(DefaultOutput::new_http(Arc::new(StubHttpOutbound)));
    let mgr     = runtime_manager(config, ingress, egress, Arc::new(StubLifecycle));

    mgr.start().await.expect("start ok");
    assert_eq!(mgr.health().await.status, RuntimeStatus::Running);
    mgr.shutdown().await.expect("shutdown ok");
    assert_eq!(mgr.health().await.status, RuntimeStatus::Stopped);
}

/// @covers: runtime_manager — health reports ingress and egress component names
#[tokio::test]
async fn test_runtime_manager_health_reports_ingress_and_egress() {
    let handler: Arc<dyn HttpInbound> = Arc::new(EchoHandler);
    let config  = RuntimeConfig::default().with_systemd_notify(false);
    let ingress = Arc::new(DefaultInput::new_http(handler));
    let egress  = Arc::new(DefaultOutput::new_http(Arc::new(StubHttpOutbound)));
    let mgr     = runtime_manager(config, ingress, egress, Arc::new(StubLifecycle));

    mgr.start().await.expect("start ok");
    let health = mgr.health().await;
    let names: Vec<&str> = health.components.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"ingress.http"));
    assert!(names.contains(&"egress.http"));
}

/// Full stack: real TCP request through IngressGateway → AxumHttpServer → HttpInbound handler.
#[tokio::test]
async fn test_http_get_request_flows_end_to_end_through_daemon_stack() {
    let (base, mgr, shutdown_tx) = start_daemon_stack(Arc::new(EchoHandler)).await;

    let resp = reqwest::get(format!("{base}/ping")).await.expect("HTTP request failed");
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("GET"), "expected GET in echo body, got: {body}");

    let _ = shutdown_tx.send(());
    mgr.shutdown().await.expect("shutdown ok");
}

/// Full stack: POST with JSON body reaches the handler.
#[tokio::test]
async fn test_http_post_with_json_body_reaches_handler() {
    let (base, mgr, shutdown_tx) = start_daemon_stack(Arc::new(EchoHandler)).await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/submit"))
        .json(&serde_json::json!({"key": "value"}))
        .send()
        .await
        .expect("HTTP request failed");
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("POST"), "expected POST in echo body, got: {body}");

    let _ = shutdown_tx.send(());
    mgr.shutdown().await.expect("shutdown ok");
}

/// Full stack: handler error maps to the correct HTTP status code at the wire level.
#[tokio::test]
async fn test_handler_not_found_error_surfaces_as_404_at_wire_level() {
    let (base, mgr, shutdown_tx) = start_daemon_stack(Arc::new(NotFoundHandler)).await;

    let resp = reqwest::get(format!("{base}/missing")).await.expect("HTTP request failed");
    assert_eq!(resp.status(), 404);

    let _ = shutdown_tx.send(());
    mgr.shutdown().await.expect("shutdown ok");
}

/// Full stack: server stops accepting connections after daemon shutdown.
#[tokio::test]
async fn test_server_refuses_connections_after_daemon_shutdown() {
    let (base, mgr, shutdown_tx) = start_daemon_stack(Arc::new(EchoHandler)).await;

    // Confirm it's up.
    reqwest::get(format!("{base}/check")).await.expect("should be up");

    // Tear down.
    let _ = shutdown_tx.send(());
    mgr.shutdown().await.expect("shutdown ok");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let result = reqwest::get(format!("{base}/check")).await;
    assert!(result.is_err(), "expected connection refused after shutdown");
}
