# Edge

> **Scope**: High-level overview only. Implementation details belong in [Developer Guide](4-development/developer_guide.md).

**Audience**: Developers, architects

## WHAT

Embeddable, library-level HTTP/gRPC dispatch stack for Rust services. Enforces the SEA (Structural Engineering Architecture) module contract at the type level — no sidecar process, no framework lock-in.

Key capabilities:
- **Transport abstraction** — consumer code imports only traits from `api/`; Axum, Tonic, and reqwest never cross the boundary
- **Seven opt-in middleware crates** — auth, retry, rate limiting, circuit breaker, caching, record/replay, mTLS; policy in TOML
- **Graceful shutdown** — SIGTERM/SIGINT handling, systemd `sd_notify`, configurable drain timeout
- **Five independent workspaces** — ingress, egress, proxy, domain, runtime; add only what you need

## WHY

| Problem | Solution |
|---------|----------|
| Sidecar proxies require a separate process (Envoy, Traefik) | Edge is a Rust library — no process overhead, embedded in the service binary |
| Framework types leak into business logic (Axum extractors in domain code) | SEA contract enforces the boundary at the type level; business logic holds `dyn HttpInbound` only |
| Middleware policy scattered as Rust literals across the codebase | All policy lives in TOML config; never hardcoded in Rust |
| Testing requires mocking the entire transport layer | Port-0 integration tests use real listeners; no transport mocks needed |

## HOW

### Architecture

```
┌──────────────────────────────────┐
│  saf/  ← sole public factory     │
│  api/  ← traits & value objects  │  ← Consumer imports these
│  core/ ← pub(crate) impls        │
└──────────────────────────────────┘
           │
           ▼
Transport (Axum / Tonic / hyper) — behind the boundary
```

### Quick Start

```rust
use std::sync::Arc;
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

### Layer Responsibilities

| Workspace | Crate prefix | Purpose |
|-----------|-------------|---------|
| `ingress/` | `swe-edge-ingress-*` | Inbound port traits — `HttpInbound`, `GrpcInbound`, `FileInbound` |
| `egress/` | `swe-edge-egress-*` | Outbound port traits — `HttpOutbound`, `GrpcOutbound`, `DatabaseGateway` + 7 middleware crates |
| `proxy/` | `swe-edge-proxy` | Dispatch facade — `Job → Router → LifecycleMonitor` |
| `domain/` | `swe-edge-domain` | Business logic contracts — `Handler → HandlerRegistry` |
| `runtime/` | `swe-edge-runtime` | Wires all layers — `RuntimeManager`, graceful shutdown, systemd notify |

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](3-architecture/architecture.md) | Block diagram, dataflow, sequence diagrams, SEA module layout |
| [Developer Guide](4-development/developer_guide.md) | Build, extend, and contribute |
| [Setup Guide](4-development/setup_guide.md) | First-time toolchain and workspace setup |
| [Testing Strategy](5-testing/testing_strategy.md) | Test categories, conventions, coverage targets |
| [Deployment Guide](6-operations/deployment_guide.md) | Consuming Edge in production |
| [Value Proposition](0-research/value_proposition.md) | Why Edge exists |

---

**Status**: Alpha
