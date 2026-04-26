# Egress Developer Guide

## Prerequisites

- Rust 1.75+ (`rustup show` to confirm)
- `cargo` on `$PATH`
- On Windows: MSVC toolchain or `x86_64-pc-windows-gnu` (for `aws-lc-sys` / `ring` build deps)
- For TLS integration tests: a local cert or use the cassette fixture path

---

## Build and test

All commands run from `egress/` — there is no root-level Cargo project.

```bash
# Build every crate
cargo build

# Build a single crate
cargo build -p swe-edge-egress-http

# Run all tests
cargo test --workspace

# Run tests for one crate
cargo test -p swe-edge-egress-http

# Run a single test function (with stdout)
cargo test test_health_check_succeeds_when_server_is_listening -- --nocapture

# Lint (zero warnings is the bar)
cargo clippy -- -D warnings

# Format check
cargo fmt --check
```

---

## Module layout at a glance

```
src/
├── api/          # Declare types and traits here
│   ├── port/     # Outbound trait definitions
│   └── value_object/  # Request/response/config types
├── core/         # Implement traits here — pub(crate) only
├── saf/          # Factory functions and Builder impl — the only public export surface
└── lib.rs        # pub use saf::*  (nothing else)
```

`core/` items are never `pub`. `api/` items are `pub(crate)` unless they are part of the crate's public surface (traits, config types, error types). `saf/` is the gate: consumers call a factory, receive `impl Trait`, and never name a `core/` type.

---

## Using the HTTP outbound

### Simplest form — no middleware

```rust
use swe_edge_egress_http::{HttpConfig, HttpOutbound, plain_http_outbound};

let client = plain_http_outbound(HttpConfig::with_base_url("https://api.example.com"))?;
let resp = client.get("/v1/resource").await?;
```

### SWE defaults — all middleware layers

```rust
use swe_edge_egress_http::{HttpOutbound, default_http_outbound};

let client = default_http_outbound()?;
let resp = client.get("https://api.example.com/v1/resource").await?;
```

### Full control — supply every layer's config

```rust
use swe_edge_egress_http::{HttpConfig, HttpOutbound, HttpOutboundConfig, http_outbound};

let config = HttpOutboundConfig {
    http:          HttpConfig::with_base_url("https://api.example.com"),
    auth:          swe_edge_egress_auth::AuthConfig::from_config(r#"
                       kind = "bearer"
                       token_env = "MY_API_TOKEN"
                   "#)?,
    retry:         swe_edge_egress_retry::RetryConfig::swe_default()?,
    rate:          swe_edge_egress_rate::RateConfig::swe_default()?,
    breaker:       swe_edge_egress_breaker::BreakerConfig::swe_default()?,
    cache:         swe_edge_egress_cache::CacheConfig::swe_default()?,
    cassette:      swe_edge_egress_cassette::CassetteConfig::swe_default()?,
    cassette_name: "my_service".into(),
    tls:           swe_edge_egress_tls::TlsConfig::swe_default()?,
};

let client = http_outbound(config)?;
```

### Sending requests

```rust
use swe_edge_egress_http::{HttpBody, HttpMethod, HttpRequest};
use serde_json::json;

// GET (convenience)
let resp = client.get("/items").await?;

// POST with JSON body
let req = HttpRequest::post("/items")
    .with_body(HttpBody::Json(json!({"name": "widget"})))
    .with_header("x-trace-id", "abc123");
let resp = client.send(req).await?;

// Full control
let req = HttpRequest {
    method:  HttpMethod::Patch,
    url:     "/items/42".into(),
    headers: Default::default(),
    query:   Default::default(),
    body:    Some(HttpBody::Raw(b"raw bytes".to_vec())),
    timeout: Some(std::time::Duration::from_secs(5)),
};
let resp = client.send(req).await?;
```

`base_url` in `HttpConfig` is prepended to any URL that does not start with `http://` or `https://`.

---

## Using the gRPC outbound

```rust
use swe_edge_egress_grpc::{GrpcOutbound, GrpcRequest, TonicGrpcClient};

let client = TonicGrpcClient::new("http://localhost:50051");   // h2c
let client = TonicGrpcClient::new("https://grpc.example.com"); // TLS

let req = GrpcRequest::new("package.Service/Method", proto_bytes)
    .with_header("authorization", "Bearer tok");

let resp = client.call_unary(req).await?;
// resp.body    — decoded response payload (gRPC framing removed)
// resp.metadata.headers — response trailer map
```

For streaming:

```rust
use futures::stream;

let messages: swe_edge_egress_grpc::GrpcMessageStream =
    Box::pin(stream::iter(vec![Ok(msg1), Ok(msg2)]));

let response_stream = client.call_stream("pkg.Svc/Method".into(), metadata, messages).await?;
```

Note: `TonicGrpcClient::call_stream` buffers the entire input stream into one HTTP/2 request body and collects the full response before returning. It is not suitable for unbounded or large streams.

---

## Config loading patterns

Every middleware crate exposes three ways to load config:

```rust
// 1. Parse from a TOML string (test overrides, consumer config files)
let cfg = RetryConfig::from_config(r#"
    max_retries = 5
    initial_interval_ms = 100
    max_interval_ms = 30000
    multiplier = 3.0
    retryable_statuses = [429, 503]
    retryable_methods = ["GET", "POST"]
"#)?;

// 2. Load the crate-shipped SWE baseline (embedded at build time)
let cfg = RetryConfig::swe_default()?;

// 3. Build with an explicit config (skipping the TOML round-trip)
let layer = swe_edge_egress_retry::Builder::with_config(cfg).build()?;

// 4. Convenience builder() that loads swe_default() internally
let layer = swe_edge_egress_retry::builder()?.build()?;
```

**Config layering convention:**

```
config/default.toml       (shipped with each crate — immutable SWE baseline)
config/application.toml   (per-workspace override — committed to repo)
application config         (loaded by the consumer at runtime)
Builder::with_config()     (test override — highest precedence)
```

---

## Adding auth to an existing client

Auth is configured at build time. The resolved credential (env var value) is captured once inside `AuthMiddleware` and reused for every request. It is not refreshed if the env var changes after construction.

```rust
use swe_edge_egress_auth::AuthConfig;

// Bearer
let auth_cfg = AuthConfig::from_config(r#"
    kind = "bearer"
    token_env = "OPENAI_API_KEY"
"#)?;

// AWS SigV4 for S3-compatible APIs
let auth_cfg = AuthConfig::from_config(r#"
    kind = "aws_sig_v4"
    access_key_env = "AWS_ACCESS_KEY_ID"
    secret_key_env = "AWS_SECRET_ACCESS_KEY"
    region = "us-east-1"
    service = "s3"
"#)?;
```

`AuthConfig::from_config` returns `Err` if an unknown field is present — this prevents inline credential typos from silently being ignored.

---

## Writing tests

### Unit tests

Tests with no I/O live inline in `#[cfg(test)]` modules. Name them `test_<action>_<condition>_<expectation>`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_for_grows_exponentially() {
        let l = RetryLayer::new(test_config());
        assert_eq!(l.backoff_for(0), Duration::from_millis(200));
        assert_eq!(l.backoff_for(1), Duration::from_millis(400));
    }
}
```

Test-only methods (helpers, accessors that expose internal state) belong in a `#[cfg(test)] impl Type { ... }` block to avoid `dead_code` warnings under `cargo clippy -D warnings`.

### Integration tests

Integration tests live in `tests/*.rs` and are named `*_int_test.rs` or `*_e2e_test.rs`. They start a real server or use a real filesystem — no mock substitutes for the system under test.

```rust
// tests/http_outbound_int_test.rs

#[tokio::test]
async fn test_send_with_json_body_sets_application_json_content_type() {
    let (port, _jh) = spawn_once(|req| async move {
        let ct = req.headers().get("content-type")...;
        Response::new(Full::new(Bytes::from(ct)))
    }).await;

    let cfg = HttpConfig::with_base_url(format!("http://127.0.0.1:{port}"));
    let client = plain_http_outbound(cfg).unwrap();
    let req = HttpRequest::post("/").with_body(HttpBody::Json(json!({})));
    let resp = client.send(req).await.unwrap();
    assert!(resp.body_as_str().contains("application/json"));
}
```

Pattern for spawning a single-connection test server: see `tests/http_outbound_int_test.rs::spawn_once` — it binds on an ephemeral port, handles one connection, and returns the port number. Multiple test functions call `spawn_once` independently; they never share server state.

### Cassette-based tests (VCR)

Cassette tests run deterministically in CI using pre-recorded fixtures. Record once locally, commit the cassette, replay in CI.

```rust
// Set mode = "auto" in test config so the cassette is created on first run,
// replayed on subsequent runs.
let toml = r#"
    mode = "auto"
    cassette_dir = "tests/cassettes"
    match_on = ["method", "url"]
    scrub_headers = ["authorization"]
    scrub_body_paths = []
"#;
let cassette_cfg = CassetteConfig::from_config(toml)?;
let cassette = Builder::with_config(cassette_cfg).build("my_test_case")?;
```

Change to `mode = "replay"` in CI. The cassette file is written as `tests/cassettes/my_test_case.yaml`.

**Never commit cassettes recorded without scrubbing.** The default `scrub_headers` list removes `authorization`, `x-api-key`, `cookie`, `set-cookie`, and `proxy-authorization` before any cassette is written to disk.

---

## Implementing a new outbound trait

All domain traits follow the same pattern. Example: adding a new `MessagingOutbound` crate.

**1. Define the trait in `api/port/`:**

```rust
// messaging/src/api/port/messaging_outbound.rs
pub trait MessagingOutbound: Send + Sync {
    fn publish(&self, msg: Message) -> BoxFuture<'_, MessagingResult<Receipt>>;
    fn health_check(&self) -> BoxFuture<'_, MessagingResult<()>>;
}
```

**2. Implement in `core/`:**

```rust
// messaging/src/core/default_messaging.rs
pub(crate) struct DefaultMessaging { ... }

impl MessagingOutbound for DefaultMessaging { ... }
```

**3. Expose via `saf/`:**

```rust
// messaging/src/saf/mod.rs
use crate::core::DefaultMessaging;
pub use crate::api::port::{MessagingOutbound, MessagingResult, MessagingError};
pub use crate::api::value_object::{Message, MessagingConfig, Receipt};

pub fn messaging_outbound(config: MessagingConfig) -> Result<impl MessagingOutbound, Error> {
    DefaultMessaging::new(config)
}
```

**4. Re-export from `lib.rs`:**

```rust
mod api; mod core; mod saf;
pub use saf::*;
```

Consumers get `impl MessagingOutbound` and never see `DefaultMessaging`.

---

## Implementing a new middleware layer

Middleware layers implement `reqwest_middleware::Middleware` and are wired into `http/src/saf/mod.rs::assemble()`.

```rust
use async_trait::async_trait;
use reqwest_middleware::{Middleware, Next, Result};

pub struct MyLayer { config: Arc<MyConfig> }

#[async_trait]
impl Middleware for MyLayer {
    async fn handle(
        &self,
        req: reqwest::Request,
        ext: &mut http::Extensions,
        next: Next<'_>,
    ) -> Result<reqwest::Response> {
        // pre-processing
        let resp = next.run(req, ext).await?;
        // post-processing
        Ok(resp)
    }
}
```

To add it to the stack:

1. Add a `with(my_layer)` call to `assemble()` in `http/src/saf/mod.rs` in the desired position.
2. Add the corresponding `MyConfig` field to `HttpOutboundConfig`.
3. Add a `MyError` variant to `HttpOutboundBuildError`.

---

## Debugging

**Inspect the resolved middleware config:**

```rust
let b = swe_edge_egress_retry::builder()?;
println!("{:#?}", b.config());
```

**Check what the cassette would record:**

Set `mode = "record"` in the cassette config during a test run, then inspect `tests/cassettes/<name>.yaml` to see the exact request/response pair. Switch back to `mode = "replay"` when done.

**Isolate a flaky test:**

```bash
cargo test test_name -- --nocapture 2>&1 | head -100
```

**Confirm zero warnings before committing:**

```bash
cargo clippy -- -D warnings && cargo fmt --check
```

---

## CI checklist

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --workspace
```

All three must pass with exit code 0. The workspace enforces `deny(unsafe_code)` and `warn(missing_docs)` at the workspace level — `clippy -D warnings` converts the doc warning to an error.
