# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Structure

This is **swe-edge**: a four-workspace Rust library stack. There is no top-level Cargo workspace — each peer directory is an independent workspace:

| Workspace | Layer | Purpose |
|-----------|-------|---------|
| `ingress/` | L1 | Inbound port contracts (HTTP, gRPC, File) |
| `egress/` | L1 | Outbound port contracts (HTTP, gRPC, Database, File, Notification, Payment) |
| `controller/` | L2 | 5-Concern orchestration facade (Job → Router → Handler → LifecycleMonitor → Gateway) |
| `http/` | L3 | 7-crate HTTP middleware workspace (auth, retry, rate, breaker, cache, cassette, tls) |

All commands must be run from within the workspace directory — there is no root-level Cargo project.

## Commands

### Ingress
```bash
cd ingress
cargo build
cargo test
cargo test --test <test_file_name>          # single integration test file
cargo test <test_fn_name> -- --nocapture    # single test function
cargo fmt --check
cargo clippy -- -D warnings
```

### Egress
```bash
cd egress
cargo build
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

### Controller
```bash
cd controller
cargo build --release
cargo test
./scripts/ci/lint.sh    # fmt + clippy
./scripts/ci/test.sh
```

### HTTP (all 7 middleware crates)
```bash
cd http
cargo build
cargo test
cargo test -p swe_http_auth     # single crate
cargo clippy -- -D warnings
```

## Architecture

### Data Flow
A request enters via `Job::run(req)` → `Router::dispatch(req)` classifies it to a handler ID → `HandlerRegistry::get(id)` retrieves the `Handler` impl → `Handler::execute(req)` runs business logic. Middleware wraps each stage and can short-circuit with `MiddlewareAction::ShortCircuit` before reaching the handler.

### Ingress / Egress Port Contracts
Ingress defines inbound port traits (`HttpInbound`, `GrpcInbound`, `FileInbound`). Egress defines outbound port traits (`HttpOutbound`, `GrpcOutbound`, `DatabaseGateway`, `FileOutbound`, `NotificationSender`, `PaymentGateway`). Factories in `saf/` return `impl Trait` — callers never name concrete types from `core/`.

### HTTP Middleware
Each middleware crate (auth, retry, rate, breaker, cache, cassette, tls) is independent. Policy lives in TOML config, never as hardcoded Rust literals. Config layers: `config/default.toml` (SWE defaults) → `http/main/config/application.toml` (workspace override) → consumer application config → `Builder::with_config(..)` (test override).

## SEA Conventions (Structural Engineering Architecture)

These are enforced structural rules — not preferences:

**Module layout** (every crate must follow):
```
src/
├── api/       # Public traits & types — all public trait declarations live here
├── core/      # Implementations — pub(crate) only, never exposed directly
├── saf/       # Service Abstraction Framework — the only public export surface
├── spi.rs     # Extension hooks for downstream consumers
└── lib.rs     # Re-exports
```

- Public surface is delegated **only** via `saf/` (SEA Rule 7)
- Default implementations stay `pub(crate)` in `core/` (Rule 50)
- Public signatures in `saf/` take/return `api/` traits (Rule 159)
- Consumers call `saf/` factories returning `impl Trait` (Rules 47, 159)

## Testing Conventions

- Integration tests: `tests/*.rs` (separate compilation unit, named `*_e2e_test.rs` or `*_int_test.rs`)
- Unit tests: inline `#[cfg(test)]` modules
- All async tests use `#[tokio::test]`
- Test naming: `test_<action>_<condition>_<expectation>`
- Fixtures: mock implementations live in test files using `saf::memory_database()`, `saf::mock_payment_gateway()`, etc.

## Linting

All workspaces enforce `#![deny(unsafe_code)]` and `#![warn(missing_docs)]` at the workspace level. All four workspaces run `cargo clippy -- -D warnings` in CI — zero warnings is the bar.

## Feature Flags (Egress)

All egress backends are opt-in. Default is no features. Available: `postgres`, `mysql`, `sqlite`, `s3`, `graphql`, `reqwest`, `email`, `stripe`, `tauri`, `full`.
