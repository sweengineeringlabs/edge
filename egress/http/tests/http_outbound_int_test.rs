//! Integration tests for the HTTP outbound domain.

use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use swe_edge_egress_http::{
    FormPart, HttpAuth, HttpBody, HttpConfig, HttpMethod, HttpOutbound, HttpOutboundError,
    HttpRequest, HttpResponse, plain_http_outbound,
};

// ─── test-server helpers ─────────────────────────────────────────────────────

/// Spawn a single-connection HTTP/1 test server.
/// Accepts one connection, calls `handler`, then exits.
/// Returns the bound port and a join handle for cleanup.
async fn spawn_once<F, Fut>(handler: F) -> (u16, tokio::task::JoinHandle<()>)
where
    F: Fn(Request<Incoming>) -> Fut + Send + Clone + 'static,
    Fut: std::future::Future<Output = Response<Full<Bytes>>> + Send,
{
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let jh = tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let io = TokioIo::new(stream);
            let _ = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req: Request<Incoming>| {
                        let handler = handler.clone();
                        async move { Ok::<_, Infallible>(handler(req).await) }
                    }),
                )
                .await;
        }
    });
    // Port is bound; OS backlog handles early client connections.
    tokio::time::sleep(Duration::from_millis(5)).await;
    (port, jh)
}

/// Spawn a TCP listener that accepts a connection but never sends a response.
/// Used to verify that per-request timeout fires correctly.
async fn spawn_hanging_server() -> (u16, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let jh = tokio::spawn(async move {
        if let Ok((_stream, _)) = listener.accept().await {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
    tokio::time::sleep(Duration::from_millis(5)).await;
    (port, jh)
}

// ─── value-object unit tests ─────────────────────────────────────────────────

#[test]
fn test_http_request_get_creates_get_method() {
    let req = HttpRequest::get("https://example.com/api");
    assert_eq!(req.method, HttpMethod::Get);
    assert_eq!(req.url, "https://example.com/api");
}

#[test]
fn test_http_request_post_creates_post_method() {
    let req = HttpRequest::post("/api/data");
    assert_eq!(req.method, HttpMethod::Post);
}

#[test]
fn test_http_request_patch_creates_patch_method() {
    let req = HttpRequest::patch("/resource");
    assert_eq!(req.method, HttpMethod::Patch);
}

#[test]
fn test_http_request_head_creates_head_method() {
    let req = HttpRequest::head("/resource");
    assert_eq!(req.method, HttpMethod::Head);
}

#[test]
fn test_http_request_options_creates_options_method() {
    let req = HttpRequest::options("/resource");
    assert_eq!(req.method, HttpMethod::Options);
}

#[test]
fn test_http_response_is_success_for_200() {
    let resp = HttpResponse::new(200, b"ok".to_vec());
    assert!(resp.is_success());
    assert!(!resp.is_client_error());
    assert!(!resp.is_server_error());
}

#[test]
fn test_http_auth_bearer_stores_token() {
    let auth = HttpAuth::bearer("tok_abc");
    assert!(matches!(auth, HttpAuth::Bearer { token } if token == "tok_abc"));
}

// ─── integration: plain_http_outbound ────────────────────────────────────────

#[tokio::test]
async fn test_health_check_succeeds_when_server_is_listening() {
    let (port, _jh) =
        spawn_once(|_req| async { Response::new(Full::new(Bytes::from("ok"))) }).await;

    let cfg = HttpConfig::with_base_url(format!("http://127.0.0.1:{port}"));
    let client = plain_http_outbound(cfg).unwrap();
    client.health_check().await.expect("health check must succeed when port is open");
}

#[tokio::test]
async fn test_health_check_fails_when_no_server_is_listening() {
    let port = {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        l.local_addr().unwrap().port()
        // listener dropped here — port freed
    };

    let cfg = HttpConfig::with_base_url(format!("http://127.0.0.1:{port}"));
    let client = plain_http_outbound(cfg).unwrap();
    let result = client.health_check().await;
    assert!(
        matches!(result, Err(HttpOutboundError::ConnectionFailed(_))),
        "expected ConnectionFailed, got {result:?}"
    );
}

#[tokio::test]
async fn test_send_with_json_body_sets_application_json_content_type() {
    let (port, _jh) = spawn_once(|req| async move {
        let ct = req
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        Response::new(Full::new(Bytes::from(ct)))
    })
    .await;

    let cfg = HttpConfig::with_base_url(format!("http://127.0.0.1:{port}"));
    let client = plain_http_outbound(cfg).unwrap();
    let req = HttpRequest::post("/").with_json(&serde_json::json!({"k": "v"})).unwrap();
    let resp = client.send(req).await.expect("JSON POST must succeed");
    let ct = String::from_utf8(resp.body).unwrap();
    assert!(
        ct.starts_with("application/json"),
        "expected application/json, server saw {ct:?}"
    );
}

#[tokio::test]
async fn test_send_with_form_body_sets_form_encoded_content_type() {
    let (port, _jh) = spawn_once(|req| async move {
        let ct = req
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        Response::new(Full::new(Bytes::from(ct)))
    })
    .await;

    let cfg = HttpConfig::with_base_url(format!("http://127.0.0.1:{port}"));
    let client = plain_http_outbound(cfg).unwrap();
    let mut form = HashMap::new();
    form.insert("key".to_string(), "value".to_string());
    let req = HttpRequest::post("/").with_form(form);
    let resp = client.send(req).await.expect("form POST must succeed");
    let ct = String::from_utf8(resp.body).unwrap();
    assert_eq!(ct, "application/x-www-form-urlencoded");
}

#[tokio::test]
async fn test_send_returns_timeout_error_when_server_hangs() {
    let (port, jh) = spawn_hanging_server().await;

    let cfg = HttpConfig {
        base_url: Some(format!("http://127.0.0.1:{port}")),
        timeout_secs: 60,
        ..Default::default()
    };
    let client = plain_http_outbound(cfg).unwrap();
    // Per-request override fires after 50 ms — much shorter than the 60 s client default.
    let req = HttpRequest::get("/").with_timeout(Duration::from_millis(50));
    let result = client.send(req).await;
    jh.abort();
    assert!(
        matches!(result, Err(HttpOutboundError::Timeout(_))),
        "expected Timeout, got {result:?}"
    );
}

#[tokio::test]
async fn test_send_returns_connection_failed_when_no_server_is_listening() {
    let port = {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        l.local_addr().unwrap().port()
    };

    let cfg = HttpConfig::with_base_url(format!("http://127.0.0.1:{port}"));
    let client = plain_http_outbound(cfg).unwrap();
    let result = client.send(HttpRequest::get("/")).await;
    assert!(
        matches!(result, Err(HttpOutboundError::ConnectionFailed(_))),
        "expected ConnectionFailed, got {result:?}"
    );
}

#[tokio::test]
async fn test_send_rejects_response_when_content_length_exceeds_max_bytes() {
    // Return an actual 150-byte body — hyper sets Content-Length: 150 automatically.
    // The client's early rejection (checking content-length before buffering) fires
    // because 150 > max_response_bytes(100).
    let (port, _jh) = spawn_once(|_req| async {
        Response::new(Full::new(Bytes::from(vec![b'x'; 150])))
    })
    .await;

    let cfg = HttpConfig {
        base_url: Some(format!("http://127.0.0.1:{port}")),
        max_response_bytes: Some(100),
        ..Default::default()
    };
    let client = plain_http_outbound(cfg).unwrap();
    let result = client.send(HttpRequest::get("/")).await;
    assert!(
        matches!(result, Err(HttpOutboundError::Internal(ref m)) if m.contains("too large")),
        "expected Internal too-large error, got {result:?}"
    );
}

#[tokio::test]
async fn test_send_applies_default_headers_to_every_request() {
    let (port, _jh) = spawn_once(|req| async move {
        let val = req
            .headers()
            .get("x-tenant-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        Response::new(Full::new(Bytes::from(val)))
    })
    .await;

    let cfg = HttpConfig {
        base_url: Some(format!("http://127.0.0.1:{port}")),
        default_headers: [("x-tenant-id".to_string(), "tenant-99".to_string())]
            .into_iter()
            .collect(),
        ..Default::default()
    };
    let client = plain_http_outbound(cfg).unwrap();
    let resp = client.send(HttpRequest::get("/")).await.expect("GET must succeed");
    let echoed = String::from_utf8(resp.body).unwrap();
    assert_eq!(echoed, "tenant-99", "default header must be forwarded on every request");
}

#[tokio::test]
async fn test_send_returns_invalid_request_error_for_bad_multipart_mime_type() {
    // Client-side validation — no network needed, but we still bind a server
    // so that if the request somehow leaves the process it fails cleanly.
    let (port, _jh) =
        spawn_once(|_req| async { Response::new(Full::new(Bytes::from("ok"))) }).await;

    let cfg = HttpConfig::with_base_url(format!("http://127.0.0.1:{port}"));
    let client = plain_http_outbound(cfg).unwrap();
    let mut req = HttpRequest::post("/");
    req.body = Some(HttpBody::Multipart(vec![FormPart {
        name:         "upload".to_string(),
        filename:     None,
        content_type: Some("not!!!a/valid/mime".to_string()),
        data:         b"bytes".to_vec(),
    }]));
    let result = client.send(req).await;
    assert!(
        matches!(result, Err(HttpOutboundError::InvalidRequest(_))),
        "expected InvalidRequest for malformed MIME type, got {result:?}"
    );
}

#[tokio::test]
async fn test_send_patch_request_reaches_server_with_correct_method() {
    let (port, _jh) = spawn_once(|req| async move {
        Response::new(Full::new(Bytes::from(req.method().as_str().to_owned())))
    })
    .await;

    let cfg = HttpConfig::with_base_url(format!("http://127.0.0.1:{port}"));
    let client = plain_http_outbound(cfg).unwrap();
    let resp = client.send(HttpRequest::patch("/")).await.expect("PATCH must succeed");
    assert_eq!(resp.body, b"PATCH");
}

#[tokio::test]
async fn test_send_head_request_reaches_server_with_correct_method() {
    let (port, _jh) = spawn_once(|req| async move {
        let method = req.method().as_str().to_owned();
        // HEAD responses carry no body; echo via a custom header instead.
        let mut resp = Response::new(Full::new(Bytes::new()));
        resp.headers_mut().insert(
            "x-received-method",
            hyper::header::HeaderValue::from_str(&method).unwrap(),
        );
        resp
    })
    .await;

    let cfg = HttpConfig::with_base_url(format!("http://127.0.0.1:{port}"));
    let client = plain_http_outbound(cfg).unwrap();
    let resp = client.send(HttpRequest::head("/")).await.expect("HEAD must succeed");
    assert_eq!(
        resp.headers.get("x-received-method").map(String::as_str),
        Some("HEAD")
    );
}

#[tokio::test]
async fn test_send_options_request_reaches_server_with_correct_method() {
    let (port, _jh) = spawn_once(|req| async move {
        Response::new(Full::new(Bytes::from(req.method().as_str().to_owned())))
    })
    .await;

    let cfg = HttpConfig::with_base_url(format!("http://127.0.0.1:{port}"));
    let client = plain_http_outbound(cfg).unwrap();
    let resp = client.send(HttpRequest::options("/")).await.expect("OPTIONS must succeed");
    assert_eq!(resp.body, b"OPTIONS");
}
