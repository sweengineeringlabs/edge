//! Live HTTP integration tests for the reqwest-backed `RestClient`.
//!
//! Skipped unless the `reqwest` feature is enabled (the file gates
//! everything inside `#[cfg(feature = "reqwest")]`). The test spins
//! up a one-shot tokio TCP listener that speaks just enough HTTP/1.1
//! to assert the client actually sends bytes — and that those bytes
//! match what the caller asked for. This is the regression test for
//! edge issue #1, where every outbound call silently returned a
//! mock 200 with no network traffic.
//!
//! Run:
//! ```text
//! cargo test -p edge-gateway --features reqwest --test http_live_int_test
//! ```

#![cfg(feature = "reqwest")]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use edge_gateway::{
    rest_client_with_base_url, GatewayError, HttpBody, HttpInbound, HttpMethod, HttpOutbound,
    HttpRequest,
};

// ============================================================================
// Hand-rolled HTTP/1.1 echo server. Reads the request, captures it for
// the test to assert against, and writes a configurable canned response.
// One-shot per spawn — the loop accepts exactly one connection and exits.
// ============================================================================

#[derive(Debug, Clone, Default)]
struct CapturedRequest {
    request_line: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl CapturedRequest {
    fn method(&self) -> &str {
        self.request_line.split_whitespace().next().unwrap_or("")
    }
    fn path(&self) -> &str {
        self.request_line.split_whitespace().nth(1).unwrap_or("")
    }
}

/// Bind to 127.0.0.1 on an OS-assigned port; return (base_url, captured_recv,
/// shutdown_sender). The server accepts one connection, captures it into
/// `captured_recv`, replies with `response_status` + `response_body`, and
/// stops. Tests that bind multiple servers can use this multiple times.
async fn spawn_oneshot(
    response_status: u16,
    response_body: &'static str,
) -> (String, oneshot::Receiver<CapturedRequest>, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("local_addr").port();
    let base_url = format!("http://127.0.0.1:{}", port);

    let (cap_tx, cap_rx) = oneshot::channel::<CapturedRequest>();
    let (stop_tx, stop_rx) = oneshot::channel::<()>();

    let cap_tx = Arc::new(parking_lot::Mutex::new(Some(cap_tx)));

    tokio::spawn(async move {
        tokio::select! {
            _ = stop_rx => {}
            accept = listener.accept() => {
                let (mut socket, _) = match accept {
                    Ok(s) => s,
                    Err(_) => return,
                };

                // Read until end of headers OR Content-Length bytes after.
                let mut buf = Vec::with_capacity(4096);
                let mut chunk = [0u8; 1024];
                loop {
                    let n = match socket.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&chunk[..n]);

                    // Naive but sufficient: stop once headers ended AND
                    // we've consumed Content-Length bytes (or 0).
                    if let Some(headers_end) = find_subseq(&buf, b"\r\n\r\n") {
                        let headers_str = String::from_utf8_lossy(&buf[..headers_end]).to_string();
                        let cl = parse_content_length(&headers_str);
                        if buf.len() >= headers_end + 4 + cl {
                            break;
                        }
                    }
                }

                let captured = parse_request(&buf);
                if let Some(tx) = cap_tx.lock().take() {
                    let _ = tx.send(captured);
                }

                let response = format!(
                    "HTTP/1.1 {} OK\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\
                     Connection: close\r\n\
                     \r\n\
                     {}",
                    response_status,
                    response_body.len(),
                    response_body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        }
    });

    (base_url, cap_rx, stop_tx)
}

fn find_subseq(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

fn parse_content_length(headers: &str) -> usize {
    for line in headers.split("\r\n") {
        if let Some(rest) = line.strip_prefix("Content-Length:").or_else(|| line.strip_prefix("content-length:")) {
            if let Ok(n) = rest.trim().parse::<usize>() {
                return n;
            }
        }
    }
    0
}

fn parse_request(buf: &[u8]) -> CapturedRequest {
    let headers_end = find_subseq(buf, b"\r\n\r\n").unwrap_or(buf.len());
    let head = String::from_utf8_lossy(&buf[..headers_end]).to_string();
    let body = if headers_end + 4 < buf.len() {
        buf[headers_end + 4..].to_vec()
    } else {
        Vec::new()
    };

    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or("").to_string();
    let mut headers = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }
    CapturedRequest { request_line, headers, body }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn live_get_actually_hits_the_socket() {
    let (base, cap_rx, _stop) = spawn_oneshot(200, r#"{"ok":true}"#).await;
    let client = rest_client_with_base_url(&base);

    let resp = client.get("/probe").await.expect("network call");
    assert_eq!(resp.status, 200, "client should report the server's status");
    assert_eq!(resp.body, br#"{"ok":true}"#);

    let captured = cap_rx.await.expect("server captured the request");
    assert_eq!(captured.method(), "GET");
    assert_eq!(captured.path(), "/probe");
}

#[tokio::test]
async fn live_post_json_sends_body_and_content_type() {
    let (base, cap_rx, _stop) = spawn_oneshot(201, r#"{"created":true}"#).await;
    let client = rest_client_with_base_url(&base);

    let resp = client
        .post_json("/users", serde_json::json!({"name": "alice"}))
        .await
        .expect("network call");
    assert_eq!(resp.status, 201);

    let captured = cap_rx.await.expect("server captured the request");
    assert_eq!(captured.method(), "POST");
    assert_eq!(captured.path(), "/users");
    assert_eq!(
        captured.headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
    let body: serde_json::Value =
        serde_json::from_slice(&captured.body).expect("body is JSON");
    assert_eq!(body["name"], "alice");
}

#[tokio::test]
async fn live_authorization_header_sent_via_request() {
    let (base, cap_rx, _stop) = spawn_oneshot(200, "{}").await;
    let client = rest_client_with_base_url(&base);

    // SAF builders return `impl HttpGateway`, which doesn't expose
    // builder-style auth. Drive the header explicitly through HttpRequest
    // — proves the wire-level auth header path works without depending
    // on the (separate) HttpAuth-builder code path.
    let req = HttpRequest::get("/me").with_header("Authorization", "Bearer test-token");
    let resp = client.send(req).await.expect("network call");
    assert_eq!(resp.status, 200);

    let captured = cap_rx.await.expect("server captured the request");
    assert_eq!(
        captured.headers.get("authorization").map(String::as_str),
        Some("Bearer test-token"),
        "Authorization header must traverse the wire"
    );
}

#[tokio::test]
async fn live_connection_refused_maps_to_connection_failed() {
    // Bind a port, immediately drop the listener so the port is closed.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    drop(listener);
    let dead_url = format!("http://{}", addr);

    let client = rest_client_with_base_url(&dead_url);
    let err = client
        .get("/anything")
        .await
        .expect_err("call to dead port must fail");
    assert!(
        matches!(err, GatewayError::ConnectionFailed(_)),
        "expected ConnectionFailed, got {:?}",
        err
    );
}

#[tokio::test]
async fn live_put_method_is_sent() {
    let (base, cap_rx, _stop) = spawn_oneshot(200, "{}").await;
    let client = rest_client_with_base_url(&base);
    let _ = client
        .send(HttpRequest {
            method: HttpMethod::Put,
            url: "/things/42".into(),
            headers: Default::default(),
            query: Default::default(),
            body: Some(HttpBody::Json(serde_json::json!({"x": 1}))),
            timeout: None,
        })
        .await
        .expect("network call");
    let captured = cap_rx.await.expect("server captured");
    assert_eq!(captured.method(), "PUT");
    assert_eq!(captured.path(), "/things/42");
}

#[tokio::test]
async fn live_delete_method_is_sent() {
    let (base, cap_rx, _stop) = spawn_oneshot(204, "").await;
    let client = rest_client_with_base_url(&base);
    let _ = client.delete("/things/42").await.expect("network call");
    let captured = cap_rx.await.expect("server captured");
    assert_eq!(captured.method(), "DELETE");
    assert_eq!(captured.path(), "/things/42");
}

#[tokio::test]
async fn live_query_params_are_sent_in_url() {
    let (base, cap_rx, _stop) = spawn_oneshot(200, "{}").await;
    let client = rest_client_with_base_url(&base);
    let req = HttpRequest::get("/search").with_query("q", "hello").with_query("page", "2");
    let _ = client.send(req).await.expect("network call");
    let captured = cap_rx.await.expect("server captured");
    let path = captured.path();
    assert!(path.starts_with("/search?"), "path: {}", path);
    // HashMap iteration is unordered — check both params present.
    assert!(path.contains("q=hello"), "path: {}", path);
    assert!(path.contains("page=2"), "path: {}", path);
}

#[tokio::test]
async fn live_raw_body_is_sent_byte_for_byte() {
    let (base, cap_rx, _stop) = spawn_oneshot(200, "{}").await;
    let client = rest_client_with_base_url(&base);
    let payload: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF];
    let req = HttpRequest::post("/binary").with_body(payload.clone(), "application/octet-stream");
    let _ = client.send(req).await.expect("network call");
    let captured = cap_rx.await.expect("server captured");
    assert_eq!(captured.body, payload, "raw body must traverse byte-for-byte");
    assert_eq!(
        captured.headers.get("content-type").map(String::as_str),
        Some("application/octet-stream")
    );
}

#[tokio::test]
async fn live_form_body_is_url_encoded() {
    let (base, cap_rx, _stop) = spawn_oneshot(200, "{}").await;
    let client = rest_client_with_base_url(&base);
    let mut form = HashMap::new();
    form.insert("user".to_string(), "alice".to_string());
    form.insert("role".to_string(), "admin".to_string());
    let req = HttpRequest::post("/form").with_form(form);
    let _ = client.send(req).await.expect("network call");
    let captured = cap_rx.await.expect("server captured");
    let body = String::from_utf8_lossy(&captured.body);
    // HashMap iteration is unordered — check both pairs present.
    assert!(body.contains("user=alice"), "body: {}", body);
    assert!(body.contains("role=admin"), "body: {}", body);
    assert_eq!(
        captured
            .headers
            .get("content-type")
            .map(String::as_str)
            .map(|s| s.to_ascii_lowercase()),
        Some("application/x-www-form-urlencoded".to_string())
    );
}

#[tokio::test]
async fn live_response_body_size_cap_rejects_oversized() {
    // Server sends 200 but advertises a Content-Length way over the
    // default 10 MiB cap — pre-stream check must reject.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let base = format!("http://{}", addr);
    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let _ = socket.read(&mut [0u8; 4096]).await;
            let resp = "HTTP/1.1 200 OK\r\n\
                        Content-Type: application/octet-stream\r\n\
                        Content-Length: 50000000\r\n\
                        Connection: close\r\n\
                        \r\n";
            let _ = socket.write_all(resp.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    let client = rest_client_with_base_url(&base);
    let err = client.get("/big").await.expect_err("must reject oversized body");
    assert!(
        matches!(err, GatewayError::ValidationError(_)),
        "expected ValidationError, got {:?}",
        err
    );
    assert!(err.to_string().contains("body"), "{}", err);
}

#[tokio::test]
async fn live_non_http_scheme_is_rejected_before_dispatch() {
    // No server needed — the scheme guard fires before any I/O.
    use edge_gateway::saf::rest_client;
    use edge_gateway::HttpConfig;
    let client = rest_client(HttpConfig::default());
    let req = HttpRequest::get("file:///etc/passwd");
    let err = client.send(req).await.expect_err("must reject file://");
    assert!(
        matches!(err, GatewayError::ValidationError(_)),
        "expected ValidationError, got {:?}",
        err
    );
    assert!(err.to_string().to_lowercase().contains("scheme"), "{}", err);
}

#[tokio::test]
async fn live_inbound_handle_can_be_invoked_directly() {
    // The HttpInbound echo path is in-process — verify it's reachable
    // through the trait import the public API now exposes.
    let client = rest_client_with_base_url("http://x.example");
    let req = HttpRequest::get("/probe");
    let resp = client.handle(req).await.expect("inbound echo");
    assert_eq!(resp.status, 200);
}

#[tokio::test]
async fn live_per_request_timeout_is_honoured() {
    // Server accepts but never replies — client must time out.
    // We must HOLD the accepted socket; if we drop it (e.g. via
    // `let _ = listener.accept().await`) the TCP close races the
    // timeout and we get ConnectionClosed instead.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        if let Ok((socket, _)) = listener.accept().await {
            // Bind the socket to a name so it lives the full sleep.
            let _kept_alive = socket;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });

    let client = rest_client_with_base_url(format!("http://{}", addr));
    // 500ms (not 150ms) for CI runners under load — small enough to
    // keep the test fast, big enough to absorb scheduling jitter.
    let req = HttpRequest::get("/slow").with_timeout(Duration::from_millis(500));
    let err = client.send(req).await.expect_err("must time out");
    assert!(
        matches!(err, GatewayError::Timeout(_)),
        "expected Timeout, got {:?}",
        err
    );
}
