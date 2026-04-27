# swe-edge Deployment Guide

**Audience**: Developers, platform engineers

## What

Guide for consuming swe-edge in a Rust application — adding it as a dependency, configuring the runtime, wiring ingress and egress ports, and operating the service in production.

## Why

swe-edge is a library, not a binary. "Deployment" means embedding it in a service binary and shipping that binary. This guide covers the end-to-end path from Cargo dependency to a running, production-hardened service.

## How

### Step 1 — Add Dependencies

swe-edge has no root workspace — add individual crates by workspace path or git tag:

```toml
[dependencies]
# Inbound HTTP server
swe-edge-ingress-http = { git = "https://github.com/sweengineeringlabs/edge", tag = "v0.1.0" }

# Outbound HTTP client (with auth + retry middleware)
swe-edge-egress-http  = { git = "https://github.com/sweengineeringlabs/edge", tag = "v0.1.0", features = ["auth", "retry"] }

# Full runtime (wires all layers + graceful shutdown)
swe-edge-runtime      = { git = "https://github.com/sweengineeringlabs/edge", tag = "v0.1.0" }

# Tokio runtime required
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }
```

Only add the workspaces you use. A service that only serves HTTP inbound has no compile dependency on `egress/` or `runtime/`.

### Step 2 — Implement Port Traits

Implement the inbound trait in your application crate. No Axum or Tonic types leak into this code:

```rust
use std::sync::Arc;
use futures::future::BoxFuture;
use swe_edge_ingress_http::{
    HttpInbound, HttpInboundResult, HttpRequest, HttpResponse, HttpHealthCheck,
};

pub struct MyService { /* domain state */ }

impl HttpInbound for MyService {
    fn handle(&self, req: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async move {
            // business logic here — no Axum types
            Ok(HttpResponse::new(200, b"hello".to_vec()))
        })
    }
    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}
```

### Step 3 — Wire the Runtime

```rust
use std::sync::Arc;
use swe_edge_runtime::{saf, DefaultInput, DefaultOutput, RuntimeConfig};
use swe_edge_egress_http::saf as http_egress;

#[tokio::main]
async fn main() {
    let config   = RuntimeConfig::from_env();
    let handler  = Arc::new(MyService::new());
    let outbound = Arc::new(http_egress::http_outbound());

    let ingress  = Arc::new(DefaultInput::new_http(handler));
    let egress   = Arc::new(DefaultOutput::new_http(outbound));
    let lifecycle = Arc::new(saf::noop_lifecycle());

    saf::run(config, ingress, egress, lifecycle)
        .await
        .expect("runtime error");
}
```

### Step 4 — Configure Middleware (TOML)

Middleware policy lives in TOML, never in Rust code. Place `config/application.toml` in your binary crate:

```toml
[auth]
kind = "bearer"
token_env = "MY_API_TOKEN"   # name of env var — never the token itself

[retry]
max_attempts = 3
initial_backoff_ms = 100
max_backoff_ms = 5000

[rate]
requests_per_second = 100
burst = 20

[breaker]
failure_threshold = 5
timeout_secs = 30
```

Secrets are read from environment variables at runtime. Never commit token values.

---

## Graceful Shutdown

`RuntimeManager` handles SIGTERM / SIGINT automatically when wired through `saf::run()`. The shutdown sequence is:

1. Stop accepting new inbound connections
2. Wait for in-flight requests to complete (up to the configured drain timeout)
3. Drop all egress connections
4. Return from `saf::run()`

For systemd services, swe-edge calls `sd_notify(READY=1)` on startup and `sd_notify(STOPPING=1)` on shutdown when the `SD_NOTIFY_SOCKET` environment variable is set.

```ini
[Service]
Type=notify
ExecStart=/usr/local/bin/my-service
Restart=on-failure
```

---

## Health Checks

The HTTP server exposes `/health` automatically. It calls `HttpInbound::health_check()` on the registered handler. Return a non-200 response to signal unhealthy:

```rust
fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
    Box::pin(async move {
        if self.db.ping().await.is_ok() {
            Ok(HttpHealthCheck::healthy())
        } else {
            Err(HttpInboundError::ServiceUnavailable("db unreachable".into()))
        }
    })
}
```

---

## TLS

### Server-side TLS (HTTPS)

```rust
use swe_edge_ingress_http::saf;
use swe_edge_ingress_tls::TlsConfig;

let tls = TlsConfig::from_pem("cert.pem", "key.pem").unwrap();
let server = saf::http_server_tls("0.0.0.0:443", handler, tls);
```

### mTLS (Client Certificate Auth)

```rust
let tls = TlsConfig::mtls("cert.pem", "key.pem", "ca.pem").unwrap();
let server = saf::http_server_tls("0.0.0.0:443", handler, tls);
```

### Egress mTLS

```toml
# config/application.toml
[tls]
cert_path = "/etc/certs/client.pem"
key_path  = "/etc/certs/client-key.pem"
ca_path   = "/etc/certs/ca.pem"
```

---

## Versioning

swe-edge follows [Semantic Versioning](https://semver.org). Reference a specific tag in `Cargo.toml` to pin to a known-good version:

```toml
swe-edge-ingress-http = { git = "https://github.com/sweengineeringlabs/edge", tag = "v0.1.0" }
```

GitHub Releases are created automatically on every `v*` tag push. Release notes are auto-generated from commit history.

---

## Related Documents

- **Architecture**: [../3-architecture/architecture.md](../3-architecture/architecture.md)
- **Developer Guide**: [../4-development/developer_guide.md](../4-development/developer_guide.md)
- **SEA Audit Reports**: [compliance/](compliance/)
