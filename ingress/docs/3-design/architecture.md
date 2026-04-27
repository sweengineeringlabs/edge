# Ingress Architecture

## Workspace overview

The ingress workspace is 4 independent Rust crates organized into two layers.

**Domain crates** — one per inbound protocol, each owning a public trait and its default server:

| Crate | Package | Purpose |
|---|---|---|
| `main` | `swe-edge-ingress` | Root re-export surface; DaemonRunner bootstrap helper |
| `http` | `swe-edge-ingress-http` | Inbound HTTP via `axum`; `HttpInbound` trait + `AxumHttpServer` |
| `grpc` | `swe-edge-ingress-grpc` | Inbound gRPC via `hyper` HTTP/2; `GrpcInbound` trait + `TonicGrpcServer` |
| `file` | `swe-edge-ingress-file` | Inbound file reads; `FileInbound` trait + `LocalFileSource` |

There are no middleware crates. Inbound adapters own the full request path — consumers extend behavior by wrapping the trait, not by inserting layers.

---

## SEA module layout

Every crate follows the same internal structure:

```
src/
├── api/          # Public type definitions and trait declarations
│   ├── port/     # Inbound trait(s) — HttpInbound, GrpcInbound, FileInbound
│   ├── value_object/  # Request/response/config value types
│   └── error.rs  # Crate error enum (where applicable)
├── core/         # Implementations — pub(crate) only, never re-exported directly
├── saf/          # Service Abstraction Façade — the only public API surface
│   └── mod.rs    # Re-exports, factory functions
├── spi.rs        # Extension hooks for downstream consumers
└── lib.rs        # pub use saf::*
```

**Rule:** `core/` types stay `pub(crate)`. External consumers receive `impl Trait` from `saf/` factories and never name a `core/` type.

---

## Public API surface

### HttpInbound

```rust
pub trait HttpInbound: Send + Sync {
    // Dispatch one inbound HTTP request; return the response.
    fn handle(&self, request: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>>;

    // Respond with a liveness signal.
    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>>;
}

pub enum HttpInboundError {
    Internal(String),
    NotFound(String),
    InvalidInput(String),
    Unavailable(String),
    Timeout(String),
    PermissionDenied(String),
}
```

**`AxumHttpServer`** — the default HTTP server:

```rust
// Bind and serve; shutdown resolves when the future completes.
pub fn new(bind: impl Into<String>, handler: Arc<dyn HttpInbound>) -> Self
pub fn with_body_limit(self, limit: usize) -> Self   // default: MAX_BODY_BYTES (4 MiB)
pub async fn serve<F: Future<Output=()> + Send + 'static>(&self, shutdown: F) -> Result<(), AxumServerError>
pub async fn serve_with_listener<F: Future<Output=()> + Send + 'static>(
    &self, listener: TcpListener, shutdown: F
) -> Result<(), AxumServerError>
```

Axum performs a graceful drain on shutdown: in-flight requests complete before the listener closes.

Error variants map to HTTP status codes: `NotFound → 404`, `InvalidInput → 400`, `PermissionDenied → 403`, `Timeout → 504`, `Unavailable → 503`, `Internal → 500`.

---

### GrpcInbound

```rust
pub trait GrpcInbound: Send + Sync {
    // Handle a single unary request.
    fn handle_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>>;

    // Handle a streaming request (client-streaming, server-streaming, or bidi).
    // Returns (response_stream, response_metadata).
    //
    // Default: reads the first message, calls handle_unary, wraps the result in a
    // single-item stream. Override for true streaming.
    fn handle_stream(
        &self, method: String, metadata: GrpcMetadata, messages: GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcInboundResult<(GrpcMessageStream, GrpcMetadata)>>;

    // Respond with a liveness signal.
    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>>;
}

pub type GrpcMessageStream = Pin<Box<dyn Stream<Item = GrpcInboundResult<Vec<u8>>> + Send>>;
pub enum GrpcInboundError {
    Internal(String), NotFound(String), InvalidArgument(String),
    Unavailable(String), DeadlineExceeded(String), PermissionDenied(String), Unimplemented(String),
}
```

**`TonicGrpcServer`** — the default gRPC server:

```rust
pub fn new(bind: impl Into<String>, handler: Arc<dyn GrpcInbound>) -> Self
pub fn with_max_message_size(self, size: usize) -> Self  // default: MAX_MESSAGE_BYTES (4 MiB)
pub async fn serve<F: Future<Output=()>>(&self, shutdown: F) -> Result<(), TonicServerError>
pub async fn serve_with_listener<F: Future<Output=()>>(
    &self, listener: TcpListener, shutdown: F
) -> Result<(), TonicServerError>
```

**No proto-gen required.** Raw bytes flow directly to `handle_unary`/`handle_stream`. The server decodes all gRPC length-prefix frames from the HTTP/2 body and passes them as a `GrpcMessageStream`. Response `GrpcMetadata.headers` are sent as HTTP/2 trailers alongside `grpc-status: 0`.

`GrpcInboundError` variants map to gRPC status codes: `NotFound → 5`, `InvalidArgument → 3`, `DeadlineExceeded → 4`, `PermissionDenied → 7`, `Unimplemented → 12`, `Unavailable → 14`, `Internal → 13`.

---

### FileInbound

```rust
pub trait FileInbound: Send + Sync {
    fn read(&self, path: &str) -> BoxFuture<'_, FileInboundResult<Vec<u8>>>;
    fn metadata(&self, path: &str) -> BoxFuture<'_, FileInboundResult<FileInfo>>;
    fn list(&self, options: ListOptions) -> BoxFuture<'_, FileInboundResult<ListResult>>;
    fn exists(&self, path: &str) -> BoxFuture<'_, FileInboundResult<bool>>;
    fn presigned_read_url(&self, path: &str, expires_in_secs: u64) -> BoxFuture<'_, FileInboundResult<PresignedUrl>>;
    fn health_check(&self) -> BoxFuture<'_, FileInboundResult<FileHealthCheck>>;

    // Default: wraps list() result as a lazy stream — no extra round-trips.
    fn list_stream(&self, options: ListOptions) -> BoxFuture<'_, FileInboundResult<FileInfoStream>>;
}
```

**Factory:** `local_file_source(base_path: impl Into<String>) -> impl FileInbound`

Constructs a local-filesystem implementation. All `path` arguments are resolved relative to `base_path`.

---

## DaemonRunner

`DaemonRunner` is the bootstrap helper for gateway processes. It does not start any server by itself — the caller's closure wires the server(s).

```rust
DaemonRunner::new(service_name: impl Into<String>) -> Self

// Fluent config
.with_bind(bind: impl Into<String>)          // default "0.0.0.0:9000"
.with_backend(backend: impl Into<String>)    // default "sidecar"
.with_obsrv_port(port: u16)                  // default DEFAULT_OBSRV_PORT
.without_observability()                     // skip ObsrvProcess sidecar

// Run — passes DaemonContext to the user closure
.run(|ctx: DaemonContext| async move { ... }) -> Result<(), Box<dyn Error>>
```

`DaemonContext` fields: `daemon_id: String` (UUID v4), `bind: SocketAddr`, `service_name: String`, `backend: String`, `obsrv_port: u16`.

When observability is enabled, `run()` initialises `swe_justobserv_context::LogContext` (structured logging context) and optionally spawns the `ObsrvProcess` sidecar on `obsrv_port`. If the sidecar binary is not present on `$PATH`, it silently falls back to in-memory observability.

---

## Request flow

### HTTP

```
HTTP client
  │
  ▼  TCP socket → TcpListener::accept
  │
  ▼  AxumHttpServer (Hyper + Axum)
  │    extract_request():
  │      - method, URL, headers, query extracted from axum::extract::Request
  │      - body read up to MAX_BODY_BYTES; JSON decoded to HttpBody::Json,
  │        form-encoded to HttpBody::Form, otherwise HttpBody::Raw
  │
  ▼  HttpInbound::handle(HttpRequest)
  │    - returns HttpResponse { status, headers, body }
  │    - on error, AxumHttpServer maps HttpInboundError → HTTP status
  │
  ▼  HTTP response to client
```

### gRPC

```
gRPC client
  │
  ▼  TCP socket (HTTP/2 prior knowledge) → TcpListener::accept
  │
  ▼  TonicGrpcServer (hyper HTTP/2)
  │    dispatch():
  │      - request URI path → GrpcRequest.method
  │      - headers → GrpcMetadata
  │      - body collected up to MAX_MESSAGE_BYTES; gRPC length-prefix frames decoded
  │      - decoded payloads emitted as GrpcMessageStream
  │
  ▼  GrpcInbound::handle_stream(method, metadata, messages)
  │    - returns (GrpcMessageStream, GrpcMetadata)
  │    - on error, maps GrpcInboundError → gRPC status code
  │
  ▼  grpc_stream_response():
  │    - each response Vec<u8> re-encoded as a gRPC length-prefix frame
  │    - GrpcMetadata.headers threaded into HTTP/2 trailers
  │    - grpc-status: 0 added to trailers
  │
  ▼  HTTP/2 response to client
```

---

## Configuration reference

### HttpConfig

```toml
base_url             = ""              # URL prefix (informational; inbound server ignores it)
timeout_secs         = 30
connect_timeout_secs = 10
max_retries          = 3               # Unused by inbound — applies to outbound only
default_headers      = {}
follow_redirects     = true
max_redirects        = 10
user_agent           = "swe-edge/0.1.0"
max_response_bytes   = 10485760        # 10 MiB hard cap on inbound request bodies
```

`HttpConfig::with_base_url(url)` constructs with all other fields defaulted.

### FileStorageConfig

```toml
storage_type = "local"   # "local" | "s3" | "gcs" | "azure" | "memory"
base_path    = "."
region       = ""        # S3 / GCS only
endpoint     = ""        # Custom endpoint for S3-compatible stores
# access_key / secret_key — not serialised to disk; set from env vars in application code
```

`FileStorageConfig::local(base_path)` and `FileStorageConfig::memory()` are the standard constructors.

---

## Dependency topology

```
main
 └─ http   (independent — AxumHttpServer, HttpInbound)
 └─ grpc   (independent — TonicGrpcServer, GrpcInbound)
 └─ file   (independent — LocalFileSource, FileInbound)

http, grpc, file
 └─ (no cross-dependencies among domain crates)
```

Domain crates are vertically independent of each other. `main` aggregates them into a single re-export surface. There are no middleware crates in ingress — inbound servers have no pluggable layer chain.
