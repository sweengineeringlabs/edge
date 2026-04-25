# swe-edge/http

HTTP middleware workspace for the SWE edge stack. Seven independent middleware crates, each implementing `reqwest_middleware::Middleware` (or a client-builder augmentation for TLS) around a config-driven policy engine.

## Crates

| Crate | Purpose |
|-------|---------|
| `swe-http-auth` | Bearer, Basic, Digest, Header, AWS SigV4 auth |
| `swe-http-breaker` | Circuit breaker — fail fast on degraded upstreams |
| `swe-http-cache` | RFC 7234 response cache with ETag revalidation and RFC 5861 SWR |
| `swe-http-cassette` | VCR-style record/replay for deterministic integration tests |
| `swe-http-rate` | Token-bucket client-side rate limiter |
| `swe-http-retry` | Exponential-backoff retry with method and status filtering |
| `swe-http-tls` | mTLS client identity (PKCS12 and PEM) |

## Usage

Each crate follows the same entry point pattern:

```rust
// 1. Start with the SWE-default baseline (always safe to build).
let mw = swe_http_auth::builder()?.build()?;

// 2. Override with TOML from your config system.
let cfg = AuthConfig::from_config(toml_text)?;
let mw = Builder::with_config(cfg).build()?;

// 3. Plug into reqwest-middleware.
let client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
    .with(mw)
    .build();
```

## Policy-in-config principle

**No hardcoded policy defaults in source code.** Every numeric threshold, timeout, list of retryable statuses, TTL, and credential reference lives in a TOML file. The crate-shipped baseline is at `<crate>/config/application.toml`. Workspace-wide overrides live in `http/main/config/application.toml`.

## Building

```bash
cargo build           # all 7 crates
cargo test            # all 7 crates
cargo test -p swe-http-auth   # single crate
cargo clippy -- -D warnings
```
