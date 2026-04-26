//! Integration tests for the HTTP inbound domain.

use swe_edge_ingress_http::{
    HttpAuth, HttpBody, HttpConfig, HttpMethod, HttpRequest, HttpResponse,
    HttpInbound, HttpInboundError, HttpInboundResult, HttpHealthCheck,
};
use futures::future::BoxFuture;

/// Minimal stub that echoes back a 200 response.
struct EchoHandler;

impl HttpInbound for EchoHandler {
    fn handle(&self, _request: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async { Ok(HttpResponse::new(200, b"ok".to_vec())) })
    }

    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

/// Stub that always returns an error.
struct FailingHandler;

impl HttpInbound for FailingHandler {
    fn handle(&self, _request: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async { Err(HttpInboundError::Unavailable("service down".into())) })
    }

    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::unhealthy("service down")) })
    }
}

#[tokio::test]
async fn test_http_inbound_handle_get_request_returns_200() {
    let handler = EchoHandler;
    let req = HttpRequest::get("https://example.com/api");
    let resp = handler.handle(req).await.unwrap();
    assert_eq!(resp.status, 200);
    assert!(resp.is_success());
}

#[tokio::test]
async fn test_http_inbound_handle_post_with_json_body_returns_200() {
    let handler = EchoHandler;
    let req = HttpRequest::post("/submit")
        .with_json(&serde_json::json!({"key": "value"}))
        .unwrap();
    let resp = handler.handle(req).await.unwrap();
    assert_eq!(resp.status, 200);
}

#[tokio::test]
async fn test_http_inbound_health_check_returns_healthy() {
    let handler = EchoHandler;
    let h = handler.health_check().await.unwrap();
    assert!(h.healthy);
}

#[tokio::test]
async fn test_http_inbound_unavailable_returns_error() {
    let handler = FailingHandler;
    let req = HttpRequest::get("/");
    let result = handler.handle(req).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HttpInboundError::Unavailable(_)));
}

#[tokio::test]
async fn test_http_inbound_failing_health_check_is_unhealthy() {
    let handler = FailingHandler;
    let h = handler.health_check().await.unwrap();
    assert!(!h.healthy);
    assert!(h.message.is_some());
}

#[test]
fn test_http_auth_bearer_constructs_correctly() {
    let auth = HttpAuth::bearer("my-token");
    assert!(matches!(auth, HttpAuth::Bearer { ref token } if token == "my-token"));
}

#[test]
fn test_http_config_default_has_timeout() {
    let cfg = HttpConfig::default();
    assert!(cfg.timeout_secs > 0);
}

#[test]
fn test_http_method_get_displays_as_get() {
    assert_eq!(HttpMethod::Get.to_string(), "GET");
}

#[test]
fn test_http_response_4xx_is_client_error() {
    let resp = HttpResponse::new(404, vec![]);
    assert!(resp.is_client_error());
    assert!(!resp.is_success());
}
