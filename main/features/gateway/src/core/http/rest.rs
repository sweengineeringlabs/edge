//! REST HTTP client implementation.
//!
//! `RestClient` is the in-tree implementation of [`HttpOutbound`] and
//! [`HttpInbound`]. Outbound traffic uses [`reqwest`] when the
//! crate-level `reqwest` feature is enabled (default for most consumers
//! through the parent workspace) and otherwise returns
//! [`GatewayError::NotSupported`] to fail fast — earlier versions of
//! this file silently returned a hardcoded 200-OK mock body regardless
//! of the feature flag, which made every outbound HTTP call appear to
//! succeed without ever leaving the process. See edge issue #1.
//!
//! The inbound impl is a thin in-process echo intended for unit tests
//! and local routing scenarios; it never touches the network and is
//! not feature-gated.

use futures::future::BoxFuture;
use std::collections::HashMap;

use crate::api::{
    http::{HttpAuth, HttpConfig, HttpRequest, HttpResponse},
    traits::{HttpGateway, HttpInbound, HttpOutbound},
    types::{GatewayError, GatewayResult, HealthCheck},
};

/// REST client implementation.
///
/// Outbound HTTP requires the `reqwest` feature (enabled by default in
/// the parent workspace). Without it, every outbound call returns
/// [`GatewayError::NotSupported`] — see crate-level docs for rationale.
#[derive(Debug, Clone)]
pub(crate) struct RestClient {
    config: HttpConfig,
    auth: HttpAuth,
    #[cfg(feature = "reqwest")]
    client: reqwest::Client,
}

impl RestClient {
    /// Creates a new REST client with the given configuration.
    pub(crate) fn new(config: HttpConfig) -> Self {
        Self {
            #[cfg(feature = "reqwest")]
            client: build_client(&config),
            config,
            auth: HttpAuth::None,
        }
    }

    /// Creates a REST client with a base URL.
    pub(crate) fn with_base_url(base_url: impl Into<String>) -> Self {
        Self::new(HttpConfig::with_base_url(base_url))
    }

    /// Sets the authentication method.
    pub(crate) fn with_auth(mut self, auth: HttpAuth) -> Self {
        self.auth = auth;
        self
    }

    /// Resolves a URL relative to the base URL.
    fn resolve_url(&self, url: &str) -> String {
        match &self.config.base_url {
            Some(base) => {
                if url.starts_with("http://") || url.starts_with("https://") {
                    url.to_string()
                } else {
                    let base = base.trim_end_matches('/');
                    let path = url.trim_start_matches('/');
                    format!("{}/{}", base, path)
                }
            }
            None => url.to_string(),
        }
    }

    /// Applies authentication to headers.
    fn apply_auth(&self, headers: &mut HashMap<String, String>) {
        match &self.auth {
            HttpAuth::None => {}
            HttpAuth::Bearer { token } => {
                headers.insert("Authorization".to_string(), format!("Bearer {}", token));
            }
            HttpAuth::Basic { username, password } => {
                use base64::Engine;
                let credentials = format!("{}:{}", username, password);
                let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
                headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
            }
            HttpAuth::ApiKey { header, key } => {
                headers.insert(header.clone(), key.clone());
            }
        }
    }
}

/// Construct the underlying `reqwest::Client` from a [`HttpConfig`].
///
/// Honours the config's `timeout_secs`, `connect_timeout_secs`,
/// `follow_redirects` / `max_redirects`, and `user_agent`. Per-request
/// timeouts override the client-level default.
#[cfg(feature = "reqwest")]
fn build_client(config: &HttpConfig) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .connect_timeout(std::time::Duration::from_secs(config.connect_timeout_secs));

    builder = if config.follow_redirects {
        builder.redirect(reqwest::redirect::Policy::limited(config.max_redirects as usize))
    } else {
        builder.redirect(reqwest::redirect::Policy::none())
    };

    if let Some(ua) = &config.user_agent {
        builder = builder.user_agent(ua);
    }

    // Panic loudly if the builder rejects the config — silent fallback
    // to `Client::default()` would drop the caller's timeouts /
    // user-agent / redirect policy, which is the kind of silent bug
    // this very issue (#1) was about. Builder failures here are rare
    // in practice (TLS init, etc.) but when they do happen we want a
    // visible error with a clear source, not a working-but-wrong client.
    builder
        .build()
        .expect("reqwest::Client::builder().build() failed — check HttpConfig TLS/timeouts/user-agent")
}

/// Map a `reqwest::Error` to the closest [`GatewayError`] variant.
///
/// reqwest's classifier flags (`is_timeout()`, `is_connect()`) only
/// inspect the outer error. Per-request timeouts and connection
/// refusals on the request-send path are wrapped in a generic
/// transport error whose actual cause has to be walked through
/// `source()`. We do that walk so the test for per-request timeouts
/// classifies as `Timeout` rather than falling through to
/// `InternalError`.
#[cfg(feature = "reqwest")]
fn map_reqwest_error(e: reqwest::Error) -> GatewayError {
    if e.is_timeout() || error_chain_indicates_timeout(&e) {
        return GatewayError::Timeout(e.to_string());
    }
    if e.is_connect() || error_chain_indicates_connect_refused(&e) {
        return GatewayError::ConnectionFailed(e.to_string());
    }
    if e.is_decode() {
        return GatewayError::SerializationError(e.to_string());
    }
    if let Some(status) = e.status() {
        return if status.is_client_error() {
            GatewayError::ValidationError(e.to_string())
        } else if status.is_server_error() {
            GatewayError::BackendError(e.to_string())
        } else {
            GatewayError::InternalError(e.to_string())
        };
    }
    GatewayError::InternalError(e.to_string())
}

/// Walk the source chain looking for a `std::io::Error` with the given
/// `ErrorKind`. Typed — doesn't depend on error message wording.
///
/// The `+ 'static` bound is required because `Error::downcast_ref` needs
/// `Any`, which needs `'static`. Both `reqwest::Error` and the inner
/// hyper / std::io errors satisfy this.
#[cfg(feature = "reqwest")]
fn chain_has_io_kind(e: &(dyn std::error::Error + 'static), kind: std::io::ErrorKind) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(e);
    while let Some(err) = current {
        if let Some(io) = err.downcast_ref::<std::io::Error>() {
            if io.kind() == kind {
                return true;
            }
        }
        current = err.source();
    }
    false
}

#[cfg(feature = "reqwest")]
fn error_chain_indicates_timeout(e: &(dyn std::error::Error + 'static)) -> bool {
    // `TimedOut` is the std::io kind emitted by tokio/hyper on
    // deadline-exceeded. Covers per-request timeouts reqwest wraps
    // into its generic transport error.
    chain_has_io_kind(e, std::io::ErrorKind::TimedOut)
}

#[cfg(feature = "reqwest")]
fn error_chain_indicates_connect_refused(e: &(dyn std::error::Error + 'static)) -> bool {
    chain_has_io_kind(e, std::io::ErrorKind::ConnectionRefused)
        || chain_has_io_kind(e, std::io::ErrorKind::ConnectionReset)
        || chain_has_io_kind(e, std::io::ErrorKind::ConnectionAborted)
}

impl HttpInbound for RestClient {
    fn handle(&self, request: HttpRequest) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        Box::pin(async move {
            // In-process echo for unit tests / local routing. Never
            // touches the network; not feature-gated for that reason.
            let body = serde_json::json!({
                "received": {
                    "method": request.method.to_string(),
                    "url": request.url,
                    "headers": request.headers,
                    "query": request.query,
                }
            });

            Ok(HttpResponse {
                status: 200,
                headers: {
                    let mut h = HashMap::new();
                    h.insert("content-type".to_string(), "application/json".to_string());
                    h
                },
                body: serde_json::to_vec(&body).unwrap_or_default(),
            })
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GatewayResult<HealthCheck>> {
        Box::pin(async move { Ok(HealthCheck::healthy()) })
    }
}

// ============================================================================
// Outbound — feature-gated.
// ============================================================================

/// Live `HttpOutbound` implementation backed by `reqwest`. Available when
/// the `reqwest` feature is enabled (the workspace default).
#[cfg(feature = "reqwest")]
impl HttpOutbound for RestClient {
    fn send(&self, mut request: HttpRequest) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        for (key, value) in &self.config.default_headers {
            request.headers.entry(key.clone()).or_insert_with(|| value.clone());
        }
        self.apply_auth(&mut request.headers);

        let url = self.resolve_url(&request.url);
        let max_response_bytes = self.config.max_response_bytes;
        Box::pin(async move {
            use crate::api::http::{HttpBody, HttpMethod};

            // Scheme allowlist — refuse anything that's not http/https
            // before the URL leaves the process. Defends against a
            // caller passing `file://` or `javascript:` and reqwest
            // doing something surprising with it.
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(GatewayError::ValidationError(format!(
                    "URL scheme not allowed (only http/https supported): {}",
                    url
                )));
            }

            let method = match request.method {
                HttpMethod::Get => reqwest::Method::GET,
                HttpMethod::Post => reqwest::Method::POST,
                HttpMethod::Put => reqwest::Method::PUT,
                HttpMethod::Delete => reqwest::Method::DELETE,
                HttpMethod::Patch => reqwest::Method::PATCH,
                HttpMethod::Head => reqwest::Method::HEAD,
                HttpMethod::Options => reqwest::Method::OPTIONS,
            };

            let mut req = self.client.request(method, &url);

            for (k, v) in &request.headers {
                req = req.header(k.as_str(), v.as_str());
            }

            if !request.query.is_empty() {
                req = req.query(&request.query);
            }

            if let Some(body) = request.body {
                req = match body {
                    HttpBody::Json(v) => req.json(&v),
                    HttpBody::Raw(bytes) => req.body(bytes),
                    HttpBody::Form(form) => req.form(&form),
                    HttpBody::Multipart(_) => {
                        return Err(GatewayError::NotSupported(
                            "multipart bodies are not yet wired through to reqwest".into(),
                        ));
                    }
                };
            }

            // Per-request timeout. We do NOT pass it to reqwest's
            // `Builder::timeout` because reqwest 0.12's resulting
            // error wraps the timeout in a generic transport-error
            // `Kind::Request` whose outer `is_timeout()` flag is
            // unreliable. Wrapping with `tokio::time::timeout`
            // instead gives an unambiguous classification path.
            let per_req_timeout = request.timeout;
            let send_fut = req.send();
            let resp = if let Some(timeout) = per_req_timeout {
                match tokio::time::timeout(timeout, send_fut).await {
                    Ok(r) => r.map_err(map_reqwest_error)?,
                    Err(_) => {
                        return Err(GatewayError::Timeout(format!(
                            "per-request timeout of {:?} exceeded for {}",
                            timeout, url
                        )));
                    }
                }
            } else {
                send_fut.await.map_err(map_reqwest_error)?
            };
            let status = resp.status().as_u16();
            let mut headers = HashMap::with_capacity(resp.headers().len());
            for (name, value) in resp.headers() {
                if let Ok(v) = value.to_str() {
                    headers.insert(name.as_str().to_string(), v.to_string());
                }
            }

            // Response-body size cap. Two layers:
            // 1. Pre-check Content-Length when the server provides it.
            // 2. Post-check during streaming so a missing/lying CL still
            //    can't OOM us.
            if let Some(cap) = max_response_bytes {
                if let Some(declared) = resp.content_length() {
                    if declared as usize > cap {
                        return Err(GatewayError::ValidationError(format!(
                            "response body too large: server declared {} bytes, cap is {}",
                            declared, cap
                        )));
                    }
                }
            }

            let body_bytes = if let Some(cap) = max_response_bytes {
                let mut buf: Vec<u8> = Vec::new();
                let mut response = resp;
                loop {
                    match response.chunk().await.map_err(map_reqwest_error)? {
                        None => break,
                        Some(chunk) => {
                            if buf.len() + chunk.len() > cap {
                                return Err(GatewayError::ValidationError(format!(
                                    "response body exceeded {}-byte cap mid-stream",
                                    cap
                                )));
                            }
                            buf.extend_from_slice(&chunk);
                        }
                    }
                }
                buf
            } else {
                resp.bytes().await.map_err(map_reqwest_error)?.to_vec()
            };

            Ok(HttpResponse {
                status,
                headers,
                body: body_bytes,
            })
        })
    }

    fn get(&self, url: &str) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        let request = HttpRequest::get(url);
        self.send(request)
    }

    fn post_json(
        &self,
        url: &str,
        body: serde_json::Value,
    ) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        let request = HttpRequest::post(url)
            .with_json(&body)
            .unwrap_or_else(|_| HttpRequest::post(url));
        self.send(request)
    }

    fn put_json(
        &self,
        url: &str,
        body: serde_json::Value,
    ) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        let request = HttpRequest::put(url)
            .with_json(&body)
            .unwrap_or_else(|_| HttpRequest::put(url));
        self.send(request)
    }

    fn delete(&self, url: &str) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        let request = HttpRequest::delete(url);
        self.send(request)
    }
}

/// Stub `HttpOutbound` impl when the `reqwest` feature is **not**
/// enabled. Every method returns [`GatewayError::NotSupported`] with a
/// pointer to the feature flag — much better than the previous silent
/// 200-OK mock that fooled every status-code-checking caller.
#[cfg(not(feature = "reqwest"))]
impl HttpOutbound for RestClient {
    fn send(&self, _request: HttpRequest) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        Box::pin(async { Err(no_reqwest_error()) })
    }

    fn get(&self, _url: &str) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        Box::pin(async { Err(no_reqwest_error()) })
    }

    fn post_json(
        &self,
        _url: &str,
        _body: serde_json::Value,
    ) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        Box::pin(async { Err(no_reqwest_error()) })
    }

    fn put_json(
        &self,
        _url: &str,
        _body: serde_json::Value,
    ) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        Box::pin(async { Err(no_reqwest_error()) })
    }

    fn delete(&self, _url: &str) -> BoxFuture<'_, GatewayResult<HttpResponse>> {
        Box::pin(async { Err(no_reqwest_error()) })
    }
}

#[cfg(not(feature = "reqwest"))]
fn no_reqwest_error() -> GatewayError {
    GatewayError::NotSupported(
        "edge-gateway built without the 'reqwest' feature; outbound HTTP is not available. \
         Enable it in your Cargo.toml: \
         edge-gateway = { ..., features = [\"reqwest\"] }"
            .to_string(),
    )
}

impl HttpGateway for RestClient {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- Pure unit tests (no network, work under any feature config) -----

    #[test]
    fn test_resolve_url_with_base() {
        let client = RestClient::with_base_url("https://api.example.com/v1");
        assert_eq!(
            client.resolve_url("/users"),
            "https://api.example.com/v1/users"
        );
    }

    #[test]
    fn test_resolve_url_passes_absolute_through() {
        let client = RestClient::with_base_url("https://api.example.com");
        assert_eq!(
            client.resolve_url("https://other.com/path"),
            "https://other.com/path"
        );
    }

    /// @covers: with_base_url
    #[test]
    fn test_with_base_url_sets_config() {
        let client = RestClient::with_base_url("http://example.com");
        assert_eq!(client.resolve_url("/api/test"), "http://example.com/api/test");
    }

    /// @covers: with_auth
    #[test]
    fn test_with_auth_sets_bearer() {
        let client = RestClient::with_base_url("http://x.com")
            .with_auth(HttpAuth::bearer("my-token"));

        let mut headers = HashMap::new();
        client.apply_auth(&mut headers);
        assert_eq!(
            headers.get("Authorization"),
            Some(&"Bearer my-token".to_string()),
        );
    }

    /// @covers: with_auth
    #[test]
    fn test_with_auth_sets_api_key() {
        let client = RestClient::with_base_url("http://x.com").with_auth(HttpAuth::ApiKey {
            header: "X-Api-Key".to_string(),
            key: "secret".to_string(),
        });

        let mut headers = HashMap::new();
        client.apply_auth(&mut headers);
        assert_eq!(headers.get("X-Api-Key"), Some(&"secret".to_string()));
    }

    /// @covers: with_auth
    #[test]
    fn test_with_auth_sets_basic_auth() {
        let client = RestClient::with_base_url("http://x.com")
            .with_auth(HttpAuth::basic("alice", "s3cret"));

        let mut headers = HashMap::new();
        client.apply_auth(&mut headers);
        let auth = headers.get("Authorization").expect("auth header set");
        assert!(auth.starts_with("Basic "), "expected Basic prefix, got {}", auth);
    }

    // ----- Inbound echo (in-process; no network) -----

    #[tokio::test]
    async fn test_inbound_handle_echoes_request_metadata() {
        let client = RestClient::with_base_url("http://x.com");
        let req = HttpRequest::get("/probe").with_query("k", "v");
        let resp = client.handle(req).await.unwrap();
        assert_eq!(resp.status, 200);
        let body: serde_json::Value = resp.json().unwrap();
        assert_eq!(body["received"]["url"], "/probe");
        assert_eq!(body["received"]["query"]["k"], "v");
    }

    // ----- Outbound, NO `reqwest` feature: every method must fail fast -----

    #[cfg(not(feature = "reqwest"))]
    mod no_reqwest {
        use super::*;

        #[tokio::test]
        async fn send_returns_not_supported_when_feature_disabled() {
            let client = RestClient::with_base_url("http://x.com");
            let err = client.get("/anything").await.unwrap_err();
            assert!(
                matches!(err, GatewayError::NotSupported(_)),
                "expected NotSupported, got: {:?}",
                err
            );
            assert!(
                err.to_string().contains("reqwest"),
                "error message should point at the feature flag, got: {}",
                err
            );
        }

        #[tokio::test]
        async fn post_returns_not_supported_when_feature_disabled() {
            let client = RestClient::with_base_url("http://x.com");
            let err = client
                .post_json("/x", serde_json::json!({}))
                .await
                .unwrap_err();
            assert!(matches!(err, GatewayError::NotSupported(_)));
        }
    }

    // ----- Outbound, WITH `reqwest`: live integration tests live in
    //       tests/http_live_int_test.rs so they can `use edge_gateway`
    //       at the public-API level rather than crate-internal types.   -----
}
