//! Axum-based HTTP server — binds a socket and delegates all requests to an
//! [`HttpInbound`] handler. Wire-level concerns (CRLF, framing, keep-alive)
//! are handled by Hyper before the request reaches this layer.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;

use crate::api::port::http_inbound::{HttpInbound, HttpInboundError};
use crate::api::value_object::{HttpBody, HttpMethod, HttpRequest, HttpResponse};

/// Hard cap on the request body size the server will read into memory.
pub const MAX_BODY_BYTES: usize = 4 * 1_024 * 1_024; // 4 MiB

/// Error returned by [`AxumHttpServer::serve`].
#[derive(Debug, thiserror::Error)]
pub enum AxumServerError {
    #[error("failed to bind to {0}: {1}")]
    Bind(String, #[source] std::io::Error),
    #[error("server error: {0}")]
    Serve(#[source] std::io::Error),
}

/// Axum-based HTTP server that routes all inbound requests through an
/// [`HttpInbound`] port.
///
/// # Usage
///
/// ```ignore
/// let server = AxumHttpServer::new("0.0.0.0:8080", handler);
/// server.serve(tokio::signal::ctrl_c().map(|_| ())).await?;
/// ```
///
/// Consumers can override the handler to swap logic, or wrap it in a
/// decorator that also implements [`HttpInbound`] to extend behaviour
/// (auth, logging, rate-limiting, etc.) without touching the server.
pub struct AxumHttpServer {
    bind:       String,
    handler:    Arc<dyn HttpInbound>,
    body_limit: usize,
}

impl AxumHttpServer {
    /// Create a server that will bind to `bind` and delegate to `handler`.
    pub fn new(bind: impl Into<String>, handler: Arc<dyn HttpInbound>) -> Self {
        Self { bind: bind.into(), handler, body_limit: MAX_BODY_BYTES }
    }

    /// Override the maximum request body size (default: [`MAX_BODY_BYTES`]).
    pub fn with_body_limit(mut self, limit: usize) -> Self {
        self.body_limit = limit;
        self
    }

    /// Bind and serve until `shutdown` resolves.
    ///
    /// Axum performs a graceful drain on shutdown: in-flight requests
    /// complete before the listener closes.
    pub async fn serve<F>(&self, shutdown: F) -> Result<(), AxumServerError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let listener = TcpListener::bind(&self.bind)
            .await
            .map_err(|e| AxumServerError::Bind(self.bind.clone(), e))?;
        self.serve_with_listener(listener, shutdown).await
    }

    /// Serve using a caller-supplied pre-bound listener.
    ///
    /// Useful when the socket must be bound before the server is
    /// constructed (e.g. pre-bind during startup for zero-downtime
    /// restarts, or port-0 allocation in tests).
    pub async fn serve_with_listener<F>(
        &self,
        listener: TcpListener,
        shutdown: F,
    ) -> Result<(), AxumServerError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        tracing::info!(
            bind = %listener.local_addr().map(|a| a.to_string()).unwrap_or_else(|_| self.bind.clone()),
            "HTTP server listening"
        );

        let handler    = self.handler.clone();
        let body_limit = self.body_limit;

        let app = Router::new().fallback(move |req: axum::extract::Request| {
            let handler = handler.clone();
            async move {
                match extract_request(req, body_limit).await {
                    Ok(http_req) => match handler.handle(http_req).await {
                        Ok(resp) => build_response(resp),
                        Err(e)   => error_response(e),
                    },
                    Err(resp) => resp,
                }
            }
        });

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(AxumServerError::Serve)
    }
}

// ── Request extraction ────────────────────────────────────────────────────────

async fn extract_request(
    req: axum::extract::Request,
    body_limit: usize,
) -> Result<HttpRequest, axum::response::Response> {
    let method  = map_method(req.method());
    let url     = req.uri().to_string();
    let query   = parse_query(req.uri().query());
    let headers = collect_headers(req.headers());
    let ct      = headers.get("content-type").map(|s| s.as_str()).unwrap_or("").to_owned();

    let bytes = axum::body::to_bytes(req.into_body(), body_limit)
        .await
        .map_err(|_| payload_too_large())?;

    let body = build_body(&bytes, &ct);
    Ok(HttpRequest { method, url, headers, query, body, timeout: None })
}

fn map_method(m: &axum::http::Method) -> HttpMethod {
    match *m {
        axum::http::Method::GET     => HttpMethod::Get,
        axum::http::Method::POST    => HttpMethod::Post,
        axum::http::Method::PUT     => HttpMethod::Put,
        axum::http::Method::PATCH   => HttpMethod::Patch,
        axum::http::Method::DELETE  => HttpMethod::Delete,
        axum::http::Method::HEAD    => HttpMethod::Head,
        axum::http::Method::OPTIONS => HttpMethod::Options,
        _                           => HttpMethod::Get,
    }
}

fn parse_query(raw: Option<&str>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(q) = raw {
        for pair in q.split('&') {
            let mut parts = pair.splitn(2, '=');
            let key   = percent_decode(parts.next().unwrap_or(""));
            let value = percent_decode(parts.next().unwrap_or(""));
            if !key.is_empty() { map.insert(key, value); }
        }
    }
    map
}

fn collect_headers(headers: &axum::http::HeaderMap) -> HashMap<String, String> {
    headers.iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|vs| (k.to_string(), vs.to_string())))
        .collect()
}

fn build_body(bytes: &bytes::Bytes, content_type: &str) -> Option<HttpBody> {
    if bytes.is_empty() { return None; }
    if content_type.contains("application/json") {
        serde_json::from_slice(bytes)
            .ok()
            .map(HttpBody::Json)
            .or_else(|| Some(HttpBody::Raw(bytes.to_vec())))
    } else if content_type.contains("application/x-www-form-urlencoded") {
        Some(HttpBody::Form(parse_form(bytes)))
    } else {
        Some(HttpBody::Raw(bytes.to_vec()))
    }
}

fn parse_form(bytes: &bytes::Bytes) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let s = std::str::from_utf8(bytes).unwrap_or("");
    for pair in s.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key   = percent_decode(parts.next().unwrap_or(""));
        let value = percent_decode(parts.next().unwrap_or(""));
        if !key.is_empty() { map.insert(key, value); }
    }
    map
}

/// Minimal percent-decode: `+` → space, `%XX` → byte.
fn percent_decode(s: &str) -> String {
    let s = s.replace('+', " ");
    let mut out   = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next();
            let h2 = chars.next();
            match (h1, h2) {
                (Some(a), Some(b)) => {
                    if let Ok(byte) = u8::from_str_radix(&format!("{a}{b}"), 16) {
                        out.push(byte as char);
                    } else {
                        // Invalid hex sequence — pass through literally.
                        out.push('%');
                        out.push(a);
                        out.push(b);
                    }
                }
                (Some(a), None) => { out.push('%'); out.push(a); }
                _               => { out.push('%'); }
            }
            continue;
        }
        out.push(c);
    }
    out
}

// ── Response building ─────────────────────────────────────────────────────────

fn build_response(resp: HttpResponse) -> axum::response::Response {
    let status = axum::http::StatusCode::from_u16(resp.status)
        .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = axum::response::Response::builder().status(status);
    for (k, v) in &resp.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }
    builder
        .body(axum::body::Body::from(resp.body))
        .unwrap_or_else(|_| internal_server_error("response build failed"))
}

fn error_response(e: HttpInboundError) -> axum::response::Response {
    let status = match &e {
        HttpInboundError::NotFound(_)         => axum::http::StatusCode::NOT_FOUND,
        HttpInboundError::InvalidInput(_)     => axum::http::StatusCode::BAD_REQUEST,
        HttpInboundError::PermissionDenied(_) => axum::http::StatusCode::FORBIDDEN,
        HttpInboundError::Timeout(_)          => axum::http::StatusCode::GATEWAY_TIMEOUT,
        HttpInboundError::Unavailable(_)      => axum::http::StatusCode::SERVICE_UNAVAILABLE,
        HttpInboundError::Internal(_)         => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    };
    axum::response::Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(axum::body::Body::from(e.to_string()))
        .unwrap_or_else(|_| internal_server_error("error response build failed"))
}

fn payload_too_large() -> axum::response::Response {
    axum::response::Response::builder()
        .status(axum::http::StatusCode::PAYLOAD_TOO_LARGE)
        .header("content-type", "text/plain; charset=utf-8")
        .body(axum::body::Body::from("request body exceeds size limit"))
        .unwrap()
}

fn internal_server_error(msg: &'static str) -> axum::response::Response {
    axum::response::Response::builder()
        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        .body(axum::body::Body::from(msg))
        .unwrap()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── map_method ────────────────────────────────────────────────────────

    #[test]
    fn test_map_method_maps_all_standard_http_verbs() {
        assert_eq!(map_method(&axum::http::Method::GET),     HttpMethod::Get);
        assert_eq!(map_method(&axum::http::Method::POST),    HttpMethod::Post);
        assert_eq!(map_method(&axum::http::Method::PUT),     HttpMethod::Put);
        assert_eq!(map_method(&axum::http::Method::PATCH),   HttpMethod::Patch);
        assert_eq!(map_method(&axum::http::Method::DELETE),  HttpMethod::Delete);
        assert_eq!(map_method(&axum::http::Method::HEAD),    HttpMethod::Head);
        assert_eq!(map_method(&axum::http::Method::OPTIONS), HttpMethod::Options);
    }

    // ── parse_query ───────────────────────────────────────────────────────

    #[test]
    fn test_parse_query_extracts_key_value_pairs() {
        let q = parse_query(Some("foo=bar&baz=qux"));
        assert_eq!(q.get("foo"), Some(&"bar".to_string()));
        assert_eq!(q.get("baz"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_parse_query_returns_empty_map_for_none() {
        assert!(parse_query(None).is_empty());
    }

    #[test]
    fn test_parse_query_handles_value_with_equals_sign() {
        let q = parse_query(Some("token=a=b"));
        assert_eq!(q.get("token"), Some(&"a=b".to_string()));
    }

    #[test]
    fn test_parse_query_decodes_percent_encoded_keys_and_values() {
        let q = parse_query(Some("na%20me=val%3D1"));
        assert_eq!(q.get("na me"), Some(&"val=1".to_string()));
    }

    // ── build_body ────────────────────────────────────────────────────────

    #[test]
    fn test_build_body_returns_none_for_empty_bytes() {
        assert!(build_body(&bytes::Bytes::new(), "application/json").is_none());
    }

    #[test]
    fn test_build_body_parses_valid_json_content_type_as_json_variant() {
        let json = serde_json::json!({"k": "v"});
        let bytes = bytes::Bytes::from(serde_json::to_vec(&json).unwrap());
        assert!(matches!(build_body(&bytes, "application/json"), Some(HttpBody::Json(_))));
    }

    #[test]
    fn test_build_body_falls_back_to_raw_for_malformed_json() {
        let bytes = bytes::Bytes::from_static(b"not-json");
        assert!(matches!(build_body(&bytes, "application/json"), Some(HttpBody::Raw(_))));
    }

    #[test]
    fn test_build_body_parses_form_encoded_content_type_as_form_variant() {
        let bytes = bytes::Bytes::from_static(b"a=1&b=2");
        assert!(matches!(build_body(&bytes, "application/x-www-form-urlencoded"), Some(HttpBody::Form(_))));
    }

    #[test]
    fn test_build_body_uses_raw_for_octet_stream_content_type() {
        let bytes = bytes::Bytes::from_static(b"\x00\x01\x02");
        assert!(matches!(build_body(&bytes, "application/octet-stream"), Some(HttpBody::Raw(_))));
    }

    // ── parse_form ────────────────────────────────────────────────────────

    #[test]
    fn test_parse_form_decodes_plus_as_space() {
        let b = bytes::Bytes::from_static(b"greeting=hello+world");
        let m = parse_form(&b);
        assert_eq!(m.get("greeting"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_parse_form_decodes_percent_encoded_characters() {
        let b = bytes::Bytes::from_static(b"path=%2Fhome%2Fuser");
        let m = parse_form(&b);
        assert_eq!(m.get("path"), Some(&"/home/user".to_string()));
    }

    // ── build_response ────────────────────────────────────────────────────

    #[test]
    fn test_build_response_maps_200_status_code() {
        let resp = build_response(HttpResponse::new(200, b"ok".to_vec()));
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    #[test]
    fn test_build_response_maps_404_status_code() {
        let resp = build_response(HttpResponse::new(404, vec![]));
        assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_build_response_falls_back_to_500_for_out_of_range_status() {
        // StatusCode::from_u16 rejects values outside 100–999.
        let resp = build_response(HttpResponse::new(1000, vec![]));
        assert_eq!(resp.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ── error_response ────────────────────────────────────────────────────

    #[test]
    fn test_error_response_maps_not_found_to_404() {
        let r = error_response(HttpInboundError::NotFound("x".into()));
        assert_eq!(r.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_response_maps_invalid_input_to_400() {
        let r = error_response(HttpInboundError::InvalidInput("bad".into()));
        assert_eq!(r.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_response_maps_permission_denied_to_403() {
        let r = error_response(HttpInboundError::PermissionDenied("no".into()));
        assert_eq!(r.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_error_response_maps_timeout_to_504() {
        let r = error_response(HttpInboundError::Timeout("slow".into()));
        assert_eq!(r.status(), axum::http::StatusCode::GATEWAY_TIMEOUT);
    }

    #[test]
    fn test_error_response_maps_unavailable_to_503() {
        let r = error_response(HttpInboundError::Unavailable("down".into()));
        assert_eq!(r.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_error_response_maps_internal_to_500() {
        let r = error_response(HttpInboundError::Internal("oops".into()));
        assert_eq!(r.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ── percent_decode ────────────────────────────────────────────────────

    #[test]
    fn test_percent_decode_leaves_plain_strings_unchanged() {
        assert_eq!(percent_decode("hello"), "hello");
    }

    #[test]
    fn test_percent_decode_converts_plus_to_space() {
        assert_eq!(percent_decode("hello+world"), "hello world");
    }

    #[test]
    fn test_percent_decode_decodes_hex_encoded_slash() {
        assert_eq!(percent_decode("%2F"), "/");
    }

    #[test]
    fn test_percent_decode_passes_through_invalid_percent_sequence() {
        assert_eq!(percent_decode("%ZZ"), "%ZZ");
    }
}
