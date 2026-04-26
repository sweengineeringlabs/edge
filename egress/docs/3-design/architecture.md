# Egress Architecture

## Workspace overview

The egress workspace is 14 independent Rust crates organized into two layers.

**Domain crates** — one per outbound protocol, each owning a public trait and its default implementation:

| Crate | Package | Purpose |
|---|---|---|
| `main` | `swe-edge-egress` | Root re-export surface; convenience assembly functions |
| `http` | `swe-edge-egress-http` | Outbound HTTP via `reqwest`; `HttpOutbound` trait |
| `grpc` | `swe-edge-egress-grpc` | Outbound gRPC via `hyper` HTTP/2; `GrpcOutbound` trait |
| `database` | `swe-edge-egress-database` | Outbound DB access; `DatabaseGateway` trait |
| `file` | `swe-edge-egress-file` | Outbound file storage; `FileOutbound` trait |
| `notification` | `swe-edge-egress-notification` | Multi-channel notifications; `NotificationSender` trait |
| `payment` | `swe-edge-egress-payment` | Payment processing; `PaymentGateway` trait |

**Middleware crates** — `reqwest-middleware` layers assembled into the HTTP stack:

| Crate | Package | Concern |
|---|---|---|
| `auth` | `swe-edge-egress-auth` | Credential injection — Bearer, Basic, Header, Digest, AWS SigV4 |
| `retry` | `swe-edge-egress-retry` | Exponential backoff retry on transient failures |
| `rate` | `swe-edge-egress-rate` | Client-side token-bucket rate limiting |
| `breaker` | `swe-edge-egress-breaker` | Circuit breaker — fail-fast on degraded upstreams |
| `cache` | `swe-edge-egress-cache` | RFC 7234 HTTP response caching (moka LRU) |
| `cassette` | `swe-edge-egress-cassette` | VCR record/replay for deterministic testing |
| `tls` | `swe-edge-egress-tls` | Client mTLS identity (PKCS#12 or PEM) |

Domain crates have no dependency on middleware crates. Middleware crates have no dependency on each other. The `http` crate's `saf/mod.rs` is the single site where all layers converge.

---

## SEA module layout

Every crate follows the same internal structure:

```
src/
├── api/          # Public type definitions and trait declarations
│   ├── port/     # Outbound trait(s) — HttpOutbound, GrpcOutbound, etc.
│   ├── value_object/  # Request/response/config value types
│   ├── error.rs  # Crate error enum
│   └── builder.rs     # Builder struct declaration (data only)
├── core/         # Implementations — pub(crate) only, never re-exported directly
├── saf/          # Service Abstraction Façade — the only public API surface
│   ├── mod.rs    # Re-exports, factory functions
│   └── builder.rs     # Builder impl (with_config, build, convenience methods)
├── spi.rs        # Extension hooks for downstream consumers
└── lib.rs        # pub use saf::*
```

**Rule:** `core/` types stay `pub(crate)`. External consumers receive `impl Trait` from `saf/` factories and never name a `core/` type.

---

## Public API surface

### HttpOutbound

```rust
pub trait HttpOutbound: Send + Sync {
    // Send any request; returns the full response.
    fn send(&self, request: HttpRequest) -> BoxFuture<'_, HttpOutboundResult<HttpResponse>>;

    // Send a GET to base_url; asserts a 2xx response. Returns Ok(()) on success,
    // Internal on non-2xx, ConnectionFailed if the host is unreachable.
    fn health_check(&self) -> BoxFuture<'_, HttpOutboundResult<()>>;

    // Default impl: builds a GET HttpRequest and calls send().
    fn get(&self, url: &str) -> BoxFuture<'_, HttpOutboundResult<HttpResponse>>;
}

pub enum HttpOutboundError { ConnectionFailed(String), Timeout(String), InvalidRequest(String), Internal(String) }
```

**Entry points** (all in `swe_edge_egress_http`):

| Function | When to use |
|---|---|
| `plain_http_outbound(HttpConfig)` | Simple use — no middleware, just reqwest |
| `default_http_outbound()` | SWE-default policy for every middleware layer |
| `http_outbound(HttpOutboundConfig)` | Full control — supply config for each layer |

---

### GrpcOutbound

```rust
pub trait GrpcOutbound: Send + Sync {
    fn call_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcOutboundResult<GrpcResponse>>;

    // Default returns GrpcOutboundError::Internal("streaming not supported").
    // Override to enable streaming.
    fn call_stream(
        &self, method: String, metadata: GrpcMetadata, messages: GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcOutboundResult<GrpcMessageStream>>;

    fn health_check(&self) -> BoxFuture<'_, GrpcOutboundResult<()>>;
}

pub type GrpcMessageStream = Pin<Box<dyn Stream<Item = GrpcOutboundResult<Vec<u8>>> + Send>>;
pub enum GrpcOutboundError { ConnectionFailed(String), Timeout(String), Internal(String), Unavailable(String) }
```

**`TonicGrpcClient` — the default implementation:**

- Uses `hyper` HTTP/2 with `hyper-rustls` (webpki roots).
- `http://` URIs → h2c (clear-text HTTP/2, prior knowledge).
- `https://` URIs → TLS (standard gRPCS).
- Default timeout: 30 seconds; override via `TonicGrpcClient::with_timeout(uri, duration)`.

**`call_stream` buffering limitation:** the `TonicGrpcClient` implementation fully buffers both the input stream (all messages encoded into one HTTP/2 request body) and the response body before returning. This is correct for small-to-medium message sets. It is not suitable for infinite or large streams; true chunked streaming requires replacing `Full<Bytes>` with a streaming body type, which is a separate workstream.

---

### DatabaseGateway

```rust
pub trait DatabaseRead {
    fn get_by_id(&self, table: &str, id: &str) -> BoxFuture<'_, DatabaseResult<Option<Record>>>;
    fn find(&self, table: &str, params: QueryParams) -> BoxFuture<'_, DatabaseResult<Vec<Record>>>;
    fn health_check(&self) -> BoxFuture<'_, DatabaseResult<()>>;
}

pub trait DatabaseWrite {
    fn insert(&self, table: &str, record: Record) -> BoxFuture<'_, DatabaseResult<WriteResult>>;
    fn update(&self, table: &str, id: &str, record: Record) -> BoxFuture<'_, DatabaseResult<WriteResult>>;
    fn delete(&self, table: &str, id: &str) -> BoxFuture<'_, DatabaseResult<()>>;
}

pub trait DatabaseGateway: DatabaseRead + DatabaseWrite {}
```

---

### FileOutbound

```rust
pub trait FileOutbound: Send + Sync {
    fn write(&self, path: &str, data: Vec<u8>, options: UploadOptions) -> BoxFuture<'_, FileResult<FileInfo>>;
    fn delete(&self, path: &str) -> BoxFuture<'_, FileResult<()>>;
    fn copy(&self, source: &str, destination: &str) -> BoxFuture<'_, FileResult<FileInfo>>;
    fn metadata(&self, path: &str) -> BoxFuture<'_, FileResult<FileInfo>>;
    fn list(&self, options: ListOptions) -> BoxFuture<'_, FileResult<ListResult>>;
    fn presigned_write_url(&self, path: &str, expires_in_secs: u64) -> BoxFuture<'_, FileResult<PresignedUrl>>;
    fn health_check(&self) -> BoxFuture<'_, FileResult<()>>;
}
```

---

## HTTP middleware assembly pipeline

**Request flow** (outermost → wire):

```
caller
  │
  ▼  auth          Credentials injected once, before any retry or cache check.
  │                Bearer/Basic/Header/Digest/SigV4 headers attached here.
  │
  ▼  retry         Catches failures from all layers below and re-dispatches.
  │                Each retry attempt re-runs rate → breaker → cache → cassette.
  │
  ▼  rate          Token-bucket per host (or global). Throttles before the
  │                breaker or cache get a chance to short-circuit.
  │
  ▼  breaker       Fails fast if the target host is tripped open.
  │                Records outcomes from cache and cassette layers too.
  │
  ▼  cache         RFC 7234. Cache hit → return without touching the wire.
  │                Cache miss → continue to cassette.
  │
  ▼  cassette      VCR intercept closest to the wire.
  │                Replay mode: match and return, or fail loudly.
  │                Record mode: pass through, persist response.
  │
  ▼  reqwest client
  │  (TLS configured on ClientBuilder before the middleware chain is built)
  │
  ▼  upstream server
```

**TLS** is not a middleware layer. It is applied to the `reqwest::ClientBuilder` via `TlsApplier::apply_to()` before the client is built, so it operates at the transport level beneath all middleware.

**Assembly** is performed in `http/src/saf/mod.rs::assemble()`. The public factories are:

```rust
// SWE defaults for every layer:
default_http_outbound() -> Result<impl HttpOutbound, HttpOutboundBuildError>

// Caller-supplied config for every layer:
http_outbound(HttpOutboundConfig) -> Result<impl HttpOutbound, HttpOutboundBuildError>

// No middleware — reqwest only:
plain_http_outbound(HttpConfig) -> Result<impl HttpOutbound, HttpOutboundBuildError>
```

`HttpOutboundBuildError` carries a `From` impl for each middleware crate's error type, so `?` in the caller propagates the correct variant.

---

## Configuration reference

Policy lives in TOML, not code. All middleware crates ship a baseline in `config/application.toml`, embedded at build time via `include_str!`. Consumers load their own TOML with `XxxConfig::from_config(toml_str)` and pass it to `Builder::with_config(cfg)`.

### HttpConfig

```toml
base_url              = ""              # URL prefix prepended to relative paths
timeout_secs          = 30             # Per-request timeout
connect_timeout_secs  = 10             # TCP connect timeout
max_retries           = 3              # Deprecated; retry middleware governs retries
default_headers       = {}             # Headers sent on every request
follow_redirects      = true
max_redirects         = 10
user_agent            = "swe-edge/0.1.0"
max_response_bytes    = 10485760       # 10 MiB hard cap; responses larger than this are rejected
```

### Auth — `AuthConfig`

Tagged on `kind`. All credential references are env-var names; inline secrets are rejected by the deserializer (`deny_unknown_fields`).

```toml
# Pass-through (SWE default)
kind = "none"

# Bearer token
kind = "bearer"
token_env = "EDGE_API_TOKEN"          # env var name, not the token itself

# HTTP Basic
kind = "basic"
user_env = "EDGE_USER"
pass_env = "EDGE_PASS"

# Custom header (e.g. x-api-key)
kind = "header"
name = "x-api-key"
value_env = "EDGE_API_KEY"

# HTTP Digest (RFC 7616)
kind = "digest"
user_env = "EDGE_USER"
password_env = "EDGE_PASS"
realm = "optional-realm-validation"   # optional

# AWS SigV4
kind = "aws_sig_v4"
access_key_env     = "AWS_ACCESS_KEY_ID"
secret_key_env     = "AWS_SECRET_ACCESS_KEY"
session_token_env  = "AWS_SESSION_TOKEN"  # optional; for STS/IMDSv2 credentials
region             = "us-east-1"
service            = "s3"
```

Credentials are resolved **at build time** (when `Builder::build()` is called). A missing env var fails construction with `Error::MissingEnvVar`; no request is ever dispatched with a missing credential.

### Retry — `RetryConfig`

```toml
max_retries         = 3       # Total extra attempts after the original; 0 = no retry
initial_interval_ms = 200     # Delay before first retry
max_interval_ms     = 10000   # Cap on any single delay
multiplier          = 2.0     # Backoff base: 200ms, 400ms, 800ms …

# Transient-server and rate-limit codes only.
# 4xx client errors are deliberately absent — retrying them is usually wrong.
retryable_statuses = [408, 425, 429, 500, 502, 503, 504]

# Idempotent verbs only. POST is omitted to prevent double-charging
# on payment and LLM completion endpoints.
retryable_methods = ["GET", "HEAD", "PUT", "DELETE"]
```

### Rate — `RateConfig`

```toml
tokens_per_second = 10    # Sustained refill rate
burst_capacity    = 20    # Maximum tokens in bucket
per_host          = true  # Each host (scheme + authority) gets its own bucket
```

### Breaker — `BreakerConfig`

```toml
failure_threshold       = 5    # Consecutive failures to trip open
half_open_after_seconds = 30   # Wait before allowing a probe request
reset_after_successes   = 3    # Probe successes required to close

# Server-fault codes only. 4xx reflects caller bugs, not upstream health.
failure_statuses = [500, 502, 503, 504]
```

State machine: **Closed** → **Open** (on `failure_threshold`) → **Half-Open** (after `half_open_after_seconds`) → **Closed** (on `reset_after_successes`).

### Cache — `CacheConfig`

```toml
default_ttl_seconds  = 300     # Fallback TTL when upstream sends no Cache-Control max-age
max_entries          = 10000   # LRU eviction at this count (moka)
respect_cache_control = true   # Honor upstream Cache-Control header
cache_private        = false   # Do not cache responses marked private
```

RFC 7234 semantics: `no-store` is always honored; `Vary: *` is never cached; `ETag` / `If-None-Match` revalidation is supported.

Known limitation: the SWR (stale-while-revalidate) background refresh cannot re-enter the `reqwest_middleware` chain because `Next<'a>` is non-`'static`. The refresh dispatches via a bare `reqwest::Client` — auth, retry, and other middleware layers are not applied on refresh requests.

### Cassette — `CassetteConfig`

```toml
mode         = "replay"                   # "replay" | "record" | "auto"
cassette_dir = "tests/cassettes"          # Relative to CARGO_MANIFEST_DIR
match_on     = ["method", "url", "body_hash"]

# Stripped before any cassette is written to disk. Prevents credential leaks in VCS.
scrub_headers = ["authorization", "x-api-key", "cookie", "set-cookie", "proxy-authorization"]

# JSON pointer paths inside request bodies to zero out before hashing.
# Handles SDK-injected trace IDs that would otherwise break exact replay.
scrub_body_paths = []
```

Modes:

| Mode | Behaviour |
|---|---|
| `replay` | Serve from disk; fail loudly on miss. Use in CI. |
| `record` | Always hit the real upstream; overwrite cassette on every call. |
| `auto` | Replay on hit; record on miss. Use during local development. |

### TLS — `TlsConfig`

```toml
# Pass-through (SWE default)
kind = "none"

# PKCS#12 bundle
kind = "pkcs12"
path = "/etc/edge/client.p12"
password_env = "EDGE_TLS_PASSWORD"   # optional

# PEM (combined cert chain + private key in one file)
kind = "pem"
path = "/etc/edge/client-combined.pem"
```

For PEM, the file must contain both the certificate chain and the private key (`cat cert.pem key.pem > combined.pem`). Passwords come from env vars only.

---

## Feature flags

All crates ship with `default = []`. No features are enabled by default.

No opt-in feature flags are currently defined. The `tracing` integration scaffolding is reserved for a future pass.

---

## Dependency topology

```
main
 └─ http ──► auth, retry, rate, breaker, cache, cassette, tls
 └─ grpc   (independent)
 └─ database (independent)
 └─ file     (independent)
 └─ notification (independent)
 └─ payment  (independent)

auth, retry, rate, breaker, cache, cassette, tls
 └─ (no cross-dependencies among middleware crates)
```

Domain crates are vertically independent of each other and of middleware. Middleware crates are horizontally independent of each other. Only `http/src/saf/mod.rs` imports from all middleware crates — it is the single composition site.
