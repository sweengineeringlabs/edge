//! Integration tests for AxumHttpServer — spins up a real listener.

use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use swe_edge_ingress_http::{
    AxumHttpServer, HttpHealthCheck, HttpInbound, HttpInboundError,
    HttpInboundResult, HttpRequest, HttpResponse,
};

// ── Stub handlers ─────────────────────────────────────────────────────────────

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

struct NotFoundHandler;

impl HttpInbound for NotFoundHandler {
    fn handle(&self, _: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async { Err(HttpInboundError::NotFound("gone".into())) })
    }
    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

struct JsonEchoHandler;

impl HttpInbound for JsonEchoHandler {
    fn handle(&self, req: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async move {
            let body = serde_json::to_vec(&serde_json::json!({
                "received": req.body.is_some()
            }))
            .unwrap_or_default();
            let mut resp = HttpResponse::new(200, body);
            resp.headers.insert("content-type".into(), "application/json".into());
            Ok(resp)
        })
    }
    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Bind on port 0, start the server, return (base_url, shutdown_trigger).
async fn start_server(handler: Arc<dyn HttpInbound>) -> (String, oneshot::Sender<()>) {
    start_server_with_limit(handler, swe_edge_ingress_http::MAX_BODY_BYTES).await
}

async fn start_server_with_limit(
    handler: Arc<dyn HttpInbound>,
    body_limit: usize,
) -> (String, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server = AxumHttpServer::new(addr.to_string(), handler).with_body_limit(body_limit);

    tokio::spawn(async move {
        let signal = async move { let _ = shutdown_rx.await; };
        let _ = server.serve_with_listener(listener, signal).await;
    });

    (base_url, shutdown_tx)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// @covers: serve — GET routed to handler, 200 returned
#[tokio::test]
async fn test_server_routes_get_request_to_handler_and_returns_200() {
    let (base, _shutdown) = start_server(Arc::new(EchoHandler)).await;
    let resp = reqwest::get(format!("{base}/hello")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let text = resp.text().await.unwrap();
    assert!(text.contains("GET"), "expected GET in echo body, got: {text}");
}

/// @covers: serve — handler NotFound error maps to HTTP 404
#[tokio::test]
async fn test_server_maps_handler_not_found_error_to_404() {
    let (base, _shutdown) = start_server(Arc::new(NotFoundHandler)).await;
    let resp = reqwest::get(format!("{base}/anything")).await.unwrap();
    assert_eq!(resp.status(), 404);
}

/// @covers: serve — JSON POST body passed to handler
#[tokio::test]
async fn test_server_passes_json_post_body_to_handler() {
    let (base, _shutdown) = start_server(Arc::new(JsonEchoHandler)).await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/data"))
        .json(&serde_json::json!({"x": 1}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["received"], true);
}

/// @covers: with_body_limit — body exceeding limit returns 413
#[tokio::test]
async fn test_server_returns_413_when_body_exceeds_configured_limit() {
    let (base, _shutdown) = start_server_with_limit(Arc::new(EchoHandler), 10).await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/upload"))
        .body(vec![0u8; 100]) // 100 bytes > 10-byte limit
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 413);
}

/// @covers: serve — bind to unavailable address returns AxumServerError::Bind
#[tokio::test]
async fn test_server_returns_bind_error_for_privileged_port() {
    let server = AxumHttpServer::new("127.0.0.1:1", Arc::new(EchoHandler));
    let err = server.serve(std::future::pending::<()>()).await;
    assert!(err.is_err());
    let msg = err.unwrap_err().to_string();
    assert!(msg.contains("failed to bind"), "unexpected error: {msg}");
}

/// @covers: serve — graceful shutdown stops accepting new connections
#[tokio::test]
async fn test_server_stops_after_shutdown_signal() {
    let (base, shutdown_tx) = start_server(Arc::new(EchoHandler)).await;

    // Confirm it's up.
    let resp = reqwest::get(format!("{base}/ping")).await.unwrap();
    assert_eq!(resp.status(), 200);

    // Signal shutdown and wait briefly.
    let _ = shutdown_tx.send(());
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // New connections should now be refused.
    let result = reqwest::get(format!("{base}/ping")).await;
    assert!(result.is_err(), "expected connection refused after shutdown");
}
