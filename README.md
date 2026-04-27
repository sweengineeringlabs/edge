# swe-edge

[![Release](https://github.com/sweengineeringlabs/edge/actions/workflows/release.yml/badge.svg)](https://github.com/sweengineeringlabs/edge/actions/workflows/release.yml)

Embeddable, library-level HTTP/gRPC dispatch stack for Rust services.
Enforces the SEA (Structural Engineering Architecture) module contract at the type level — no sidecar, no framework lock-in.

## Overview

swe-edge is a set of five independent Rust workspaces that sit between your transport layer and your business logic:

| Workspace | Crate prefix | Role |
|-----------|-------------|------|
| `ingress/` | `swe-edge-ingress-*` | Inbound port traits — `HttpInbound`, `GrpcInbound`, `FileInbound` |
| `egress/` | `swe-edge-egress-*` | Outbound port traits — `HttpOutbound`, `GrpcOutbound`, `DatabaseGateway`, `NotificationSender`, `PaymentGateway` |
| `proxy/` | `swe-edge-proxy` | Dispatch facade — `Job → Router → LifecycleMonitor` |
| `domain/` | `swe-edge-domain` | Business logic contracts — `Handler → HandlerRegistry` |
| `runtime/` | `swe-edge-runtime` | Wires all layers — `RuntimeManager`, graceful shutdown, systemd notify |

Application code imports only traits from `api/` and calls factories from `saf/`. No Axum, Tonic, or reqwest types cross the boundary.

## Quick Start

Add the crates you need to your `Cargo.toml`:

```toml
# Inbound HTTP server
[dependencies]
swe-edge-ingress-http = { git = "https://github.com/sweengineeringlabs/edge", tag = "v0.1.0" }

# Outbound HTTP client with middleware
swe-edge-egress-http = { git = "https://github.com/sweengineeringlabs/edge", tag = "v0.1.0" }

# Full runtime (wires ingress + egress + lifecycle)
swe-edge-runtime = { git = "https://github.com/sweengineeringlabs/edge", tag = "v0.1.0" }
```

Implement the inbound trait and pass it to the factory:

```rust
use std::sync::Arc;
use futures::future::BoxFuture;
use swe_edge_ingress_http::{HttpInbound, HttpInboundResult, HttpRequest, HttpResponse, HttpHealthCheck};
use swe_edge_ingress_http::saf;

struct MyHandler;

impl HttpInbound for MyHandler {
    fn handle(&self, req: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async move { Ok(HttpResponse::new(200, b"ok".to_vec())) })
    }
    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

#[tokio::main]
async fn main() {
    let server = saf::http_server("0.0.0.0:8080", Arc::new(MyHandler));
    server.serve(tokio::signal::ctrl_c().map(|_| ())).await.unwrap();
}
```

## HTTP Middleware (Egress)

Seven opt-in middleware crates compose via `reqwest-middleware`:

| Crate | Feature flag | Purpose |
|-------|-------------|---------|
| `swe-edge-egress-auth` | `auth` | Bearer, Basic, API key, AWS SigV4 |
| `swe-edge-egress-retry` | `retry` | Exponential backoff with jitter |
| `swe-edge-egress-rate` | `rate` | Token-bucket rate limiting |
| `swe-edge-egress-breaker` | `breaker` | Circuit breaker |
| `swe-edge-egress-cache` | `cache` | Response caching |
| `swe-edge-egress-cassette` | `cassette` | Record/replay for tests |
| `swe-edge-egress-tls` | `tls` | mTLS client certificates |

Policy lives in TOML config, never as hardcoded Rust literals.

## Architecture

```
Application code
      │  (imports api/ traits only)
      ▼
┌─────────────────────────────────────┐
│  saf/  ← only public export surface │
│  api/  ← traits & value objects     │
│  core/ ← pub(crate) implementations │
└─────────────────────────────────────┘
      │
      ▼
Transport (Axum / Tonic / hyper) — behind the boundary
```

Each workspace follows SEA module layout: `api/` → public contracts, `core/` → `pub(crate)` implementations, `saf/` → sole public factory surface. Consumers never name concrete types.

## Building

There is no root Cargo workspace. Build each peer independently:

```bash
cd ingress && cargo build && cargo test
cd egress  && cargo build && cargo test
cd proxy   && cargo build && cargo test
cd domain  && cargo build && cargo test
cd runtime && cargo build && cargo test
```

## Documentation

- [Market research & rationale](docs/0-research/rationale.md)
- [Crate naming conventions](docs/4-development/conventions/crate-naming.md)
- [SEA audit reports](docs/6-operations/compliance/)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
