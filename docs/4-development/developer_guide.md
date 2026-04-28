# Edge Developer Guide

**Audience**: Developers, contributors

## What

Guide for developing and extending Edge — an embeddable, library-level HTTP/gRPC dispatch stack for Rust services enforcing the SEA module contract.

## Why

Contributors need a clear entry point to understand the multi-workspace build process, SEA module layout, and the extension model for adding new port traits, middleware, or transport implementations.

## How

### Build & Test

swe-edge has five independent Rust workspaces. There is no root `Cargo.toml`. Build and test each workspace separately:

```bash
# Ingress
cd ingress
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check

# Egress
cd egress
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check

# Proxy
cd proxy
cargo build
cargo test
cargo clippy -- -D warnings

# Domain
cd domain
cargo build
cargo test
cargo clippy -- -D warnings

# Runtime
cd runtime
cargo build
cargo test
cargo clippy -- -D warnings
```

### Run a Single Test

```bash
# Single integration test file
cargo test --test axum_server_int_test

# Single test function with output
cargo test test_server_routes_get_request_to_handler_and_returns_200 -- --nocapture

# All tests in a specific crate
cargo test -p swe-edge-ingress-http
```

### Project Structure

Every crate follows the SEA module layout:

```
src/
├── api/       # Public traits and value objects
├── core/      # pub(crate) implementations — never exported directly
├── saf/       # Sole public factory surface — returns impl Trait
└── lib.rs     # pub use saf::*
```

### Adding a New Inbound Port Trait

1. Create `src/api/port.rs` in the relevant ingress crate with `pub trait MyInbound`
2. Add value objects to `src/api/value_object.rs`
3. Implement the concrete server in `src/core/server/my_server.rs` as `pub(crate) struct DefaultMyServer`
4. Add a factory to `src/saf/mod.rs`:
   ```rust
   pub fn my_server(addr: impl Into<String>, handler: Arc<dyn MyInbound>) -> impl MyInbound {
       DefaultMyServer::new(addr, handler)
   }
   ```
5. Re-export the trait and value objects from `saf/mod.rs`
6. Write integration tests in `tests/my_server_int_test.rs`

### Adding a New Outbound Port Trait

1. Create `src/api/port.rs` in the relevant egress crate with `pub trait MyOutbound`
2. Add request/response value objects to `src/api/value_object.rs`
3. Implement the concrete client in `src/core/client/my_client.rs`
4. Add error types to `src/api/error.rs`
5. Register in `saf/mod.rs` and re-export the trait

### Adding a New Egress Middleware

Each middleware is an independent crate under `egress/`. Pattern:

1. Create `egress/my_middleware/Cargo.toml` and add to `egress/Cargo.toml` workspace members
2. Implement `reqwest_middleware::Middleware` in `src/core/middleware.rs`
3. Define config struct in `src/api/config.rs` (deserializes from TOML — no hardcoded values)
4. Expose a builder in `src/saf/mod.rs`
5. Wire into `DefaultHttpOutbound` in `egress/http/src/core/outbound.rs`

Policy always lives in TOML config (`config/default.toml` → `config/application.toml` → test override). Never hardcode policy as Rust literals.

### Naming Conventions

| Concept | Rule | Example |
|---------|------|---------|
| Port traits | `pub trait DomainNoun` | `HttpInbound`, `GrpcOutbound` |
| Default implementations | `pub(crate) struct DefaultDomainNoun` | `DefaultHttpOutbound` |
| Factory functions | `pub fn noun(...)` in `saf/` | `saf::http_server(...)` |
| Test functions | `test_<action>_<condition>_<expectation>` | `test_send_request_with_timeout_returns_error` |
| Error enums | `<Domain>Error` | `HttpInboundError`, `GrpcOutboundError` |

### Testing Conventions

- Integration tests: `tests/*_int_test.rs` (hits a real listener/server)
- E2E tests: `tests/*_e2e_test.rs` (full binary or cross-workspace)
- Unit tests: inline `#[cfg(test)]` modules
- All async tests use `#[tokio::test]`
- Stub handlers in test files implement the trait directly — no mocking frameworks

```rust
// Example stub handler pattern
struct EchoHandler;
impl HttpInbound for EchoHandler {
    fn handle(&self, req: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async move { Ok(HttpResponse::new(200, req.url.into_bytes())) })
    }
    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}
```

### Linting

All workspaces enforce at the workspace level:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
```

Zero clippy warnings is the CI bar. Run `cargo clippy -- -D warnings` before every commit.

### Feature Flags (Egress)

All egress backends are opt-in via feature flags:

```toml
[features]
default    = []
postgres   = ["sqlx/postgres"]
mysql      = ["sqlx/mysql"]
sqlite     = ["sqlx/sqlite"]
s3         = ["aws-sdk-s3"]
email      = ["lettre"]
stripe     = ["stripe-rust"]
tauri      = ["tauri"]
full       = ["postgres", "mysql", "sqlite", "s3", "email", "stripe", "tauri"]
```

Enable only what you need:

```toml
swe-edge-egress-database = { git = "...", features = ["postgres"] }
```
