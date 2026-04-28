# Edge Testing Strategy

**Audience**: Developers, contributors

## Test Strategy

> Per ISO/IEC/IEEE 29119-3:2021

Requirements-based testing approach. Every port trait implementation has integration tests covering the happy path, error paths, and boundary conditions. Tests use real network listeners (port 0) — no mocking of the transport layer.

| Attribute | Value |
|-----------|-------|
| **Test Strategy** | Requirements-based, black-box integration |
| **Test Scope** | All port traits, concrete implementations, middleware, runtime wiring |
| **Entry Criteria** | Code compiles with zero warnings (`cargo check`) |
| **Exit Criteria** | All tests pass, zero clippy warnings (`-D warnings`), `cargo fmt --check` clean |

---

## Test Categories

| Category | Count | Location | Purpose |
|----------|-------|----------|---------|
| Unit | ~1,200 | `src/` inline `#[cfg(test)]` | Value objects, encoding/decoding, config parsing, error types |
| Integration | ~380 | `tests/*_int_test.rs` | Real listeners, real connections, trait implementations |
| E2E | ~0 | `tests/*_e2e_test.rs` | Cross-workspace wiring (planned) |

---

## Test Pyramid

```
           ┌──────────┐
           │   E2E    │  0 — planned: full runtime startup + teardown
           └────┬─────┘
          ┌─────┴──────┐
          │Integration │  ~380 — real TCP listeners, real HTTP/gRPC/TLS connections
          └─────┬──────┘
         ┌──────┴───────┐
         │    Unit      │  ~1,200 — value objects, frame encoding, config, error types
         └──────────────┘
```

---

## Per-Workspace Coverage

### ingress

| Test File | Crate | What it covers |
|-----------|-------|---------------|
| `axum_server_int_test.rs` | `swe-edge-ingress-http` | HTTP routing, 404 mapping, JSON body, body-limit 413, bind error, graceful shutdown |
| `tonic_grpc_server_int_test.rs` | `swe-edge-ingress-grpc` | gRPC unary, streaming, metadata propagation, TLS, mTLS |
| Inline unit tests | `swe-edge-ingress-http` | `HttpRequest`, `HttpResponse`, `HttpInboundError` value objects |

### egress

| Test File | Crate | What it covers |
|-----------|-------|---------------|
| `grpc_outbound_int_test.rs` | `swe-edge-egress-grpc` | Unary call, streaming, timeout, connection failure, grpc-status error, trailers |
| `http_outbound_int_test.rs` | `swe-edge-egress-http` | Send request, auth injection, retry on 5xx, rate limiting, circuit breaker, cache hit/miss |
| `builder_e2e_test.rs` | `swe-edge-egress-auth` | Auth middleware builder, all auth kinds (Bearer/Basic/ApiKey/AwsSigV4) |
| Inline unit tests | all middleware | Config deserialization, error types, frame encode/decode |

### proxy / domain

| Test File | What it covers |
|-----------|---------------|
| Inline unit tests | `Job::run`, `Router::dispatch`, `HandlerRegistry::get`, `Handler::execute`, error propagation |

### runtime

| Test File | What it covers |
|-----------|---------------|
| `runtime_manager_int_test.rs` | Full `RuntimeManager` start/stop cycle, health check, graceful shutdown, no-ingress config |
| Inline unit tests | `DefaultInput` / `DefaultOutput` builders, `RuntimeConfig` loading |

---

## Test Conventions

### Naming

All test functions follow `test_<action>_<condition>_<expectation>`:

```
test_server_routes_get_request_to_handler_and_returns_200
test_server_returns_413_when_body_exceeds_configured_limit
test_call_unary_returns_timeout_error_when_server_stalls
test_health_check_fails_when_no_server_is_listening
```

### Integration Test Pattern

Integration tests bind on port 0 (OS-assigned), return the base URL and a shutdown trigger:

```rust
async fn start_server(handler: Arc<dyn HttpInbound>) -> (String, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server.serve_with_listener(listener, async move { let _ = shutdown_rx.await; }).await
    });
    (format!("http://{addr}"), shutdown_tx)
}
```

### Stub Handler Pattern

Stub trait implementations live inline in the test file — no mocking framework:

```rust
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
```

### Platform Notes

- All tests run on Linux and Windows
- Tests that require a bound port use ephemeral ports (port 0) — never hardcoded ports
- The bind-failure test holds an ephemeral socket and attempts a second bind on the same address (cross-platform guaranteed failure)
- Cryptographic tests call `rustls::crypto::aws_lc_rs::default_provider().install_default().ok()` before constructing TLS clients

---

## Coverage Targets

| Metric | Target | Status |
|--------|--------|--------|
| Port trait implementations | Every impl has happy + error + boundary integration tests | Met |
| Middleware | Every middleware has builder + policy + inject integration tests | Met |
| Runtime | Start/stop cycle, health check, graceful shutdown | Met |
| Clippy | Zero warnings with `-D warnings` across all 5 workspaces | Met |
| New traits | Integration tests required before merge | Enforced by review |

---

## Test Procedures

| Procedure | Command | Environment | Order |
|-----------|---------|-------------|-------|
| Smoke | `cargo check` | Local / CI | First |
| Unit | `cargo test --lib` | Local / CI | After smoke |
| Integration | `cargo test` | Local / CI | After unit |
| Lint | `cargo clippy -- -D warnings` | CI | Parallel with tests |
| Format | `cargo fmt --check` | CI | Parallel with tests |
| Single crate | `cargo test -p swe-edge-ingress-http` | Local | As needed |

---

## Related Documents

- **Architecture**: [architecture.md](../3-architecture/architecture.md)
- **Developer Guide**: [developer_guide.md](../4-development/developer_guide.md)
- **SEA Audit Reports**: [compliance/](../6-operations/compliance/)
