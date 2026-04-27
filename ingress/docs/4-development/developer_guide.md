# Ingress Developer Guide

## Prerequisites

- Rust 1.75+ (`rustup show` to confirm)
- `cargo` on `$PATH`
- On Windows: MSVC toolchain or `x86_64-pc-windows-gnu`

---

## Build and test

All commands run from `ingress/` — there is no root-level Cargo project.

```bash
# Build every crate
cargo build

# Build a single crate
cargo build -p swe-edge-ingress-http

# Run all tests
cargo test --workspace

# Run tests for one crate
cargo test -p swe-edge-ingress-http

# Run a single test function (with stdout)
cargo test test_server_routes_get_request_to_handler_and_returns_200 -- --nocapture

# Lint (zero warnings is the bar)
cargo clippy -- -D warnings

# Format check
cargo fmt --check

# Run the HTTP echo example
cargo run -p swe-edge-ingress --example http_echo

# Run the gRPC echo example
cargo run -p swe-edge-ingress --example grpc_echo
```

---

## Module layout at a glance

```
src/
├── api/          # Declare types and traits here
│   ├── port/     # Inbound trait definitions
│   └── value_object/  # Request/response/config types
├── core/         # Implement traits here — pub(crate) only
├── saf/          # Factory functions — the only public export surface
├── spi.rs        # Extension hooks for downstream consumers
└── lib.rs        # pub use saf::*  (nothing else)
```

`core/` items are never `pub`. `api/` items are `pub(crate)` unless they are part of the crate's public surface (traits, config types, error types). `saf/` is the gate: consumers call a factory or instantiate a server directly, receive `impl Trait`, and never name a `core/` type.

---

## Implementing HttpInbound

### 1. Define the handler struct

```rust
use std::sync::Arc;

use swe_edge_ingress_http::{
    AxumHttpServer, HttpHealthCheck, HttpInbound, HttpInboundError,
    HttpInboundResult, HttpRequest, HttpResponse,
};

struct MyHandler {
    // fields shared across requests — use Arc<Mutex<_>> if mutable
}
```

### 2. Implement the trait

```rust
impl HttpInbound for MyHandler {
    fn handle(&self, req: HttpRequest) -> futures::future::BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async move {
            // req.method  — HttpMethod (Get, Post, Put, Patch, Delete, …)
            // req.url     — String, path + query string as received
            // req.headers — HashMap<String, String>
            // req.query   — HashMap<String, String> (parsed query params)
            // req.body    — Option<HttpBody>  (Json / Form / Raw after decoding)
            // req.auth    — Option<HttpAuth>  (Bearer / Basic extracted from Authorization header)

            match req.method {
                HttpMethod::Get => {
                    let resp = HttpResponse::new(200, b"hello".to_vec());
                    Ok(resp)
                }
                _ => Err(HttpInboundError::InvalidInput("only GET is supported".into())),
            }
        })
    }

    fn health_check(&self) -> futures::future::BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}
```

`HttpResponse::new(status, body)` constructs a response with an empty header map. Set headers directly:

```rust
let mut resp = HttpResponse::new(200, body_bytes);
resp.headers.insert("content-type".into(), "application/json; charset=utf-8".into());
```

### 3. Wire with AxumHttpServer

```rust
use std::sync::Arc;

let handler = Arc::new(MyHandler { /* ... */ });
let server  = AxumHttpServer::new("0.0.0.0:8080", handler);

// With a Ctrl+C shutdown signal
server.serve(async {
    let _ = tokio::signal::ctrl_c().await;
}).await?;

// With a body limit override (default is MAX_BODY_BYTES = 4 MiB)
let server = AxumHttpServer::new("0.0.0.0:8080", handler)
    .with_body_limit(1024 * 1024); // 1 MiB
```

**Error → HTTP status mapping** (handled automatically by `AxumHttpServer`):

| `HttpInboundError` variant | HTTP status |
|---|---|
| `NotFound` | 404 |
| `InvalidInput` | 400 |
| `PermissionDenied` | 403 |
| `Timeout` | 504 |
| `Unavailable` | 503 |
| `Internal` | 500 |

---

## Implementing GrpcInbound

### 1. Unary-only handler (most common case)

Override only `handle_unary`. The default `handle_stream` implementation reads the first message, calls `handle_unary`, and wraps the result in a single-item stream.

```rust
use swe_edge_ingress_grpc::{
    GrpcHealthCheck, GrpcInbound, GrpcInboundResult,
    GrpcMetadata, GrpcRequest, GrpcResponse,
};

struct MyHandler;

impl GrpcInbound for MyHandler {
    fn handle_unary(
        &self,
        req: GrpcRequest,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            // req.method   — String, e.g. "/package.Service/Method"
            // req.body     — Vec<u8>, raw payload (gRPC framing already stripped)
            // req.metadata — GrpcMetadata { headers: HashMap<String, String> }

            // Parse your proto here; return a serialized proto response.
            Ok(GrpcResponse {
                body:     req.body, // echo example
                metadata: GrpcMetadata::default(),
            })
        })
    }

    fn health_check(&self) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async { Ok(GrpcHealthCheck::healthy()) })
    }
}
```

### 2. Streaming handler (override handle_stream)

```rust
use futures::StreamExt as _;
use swe_edge_ingress_grpc::GrpcMessageStream;

fn handle_stream(
    &self,
    _method: String,
    _metadata: GrpcMetadata,
    messages: GrpcMessageStream,
) -> futures::future::BoxFuture<'_, GrpcInboundResult<(GrpcMessageStream, GrpcMetadata)>> {
    Box::pin(async move {
        // Collect all input frames.
        let frames: Vec<_> = messages.collect().await;

        // Build a response stream.
        let out: GrpcMessageStream = Box::pin(futures::stream::iter(
            frames.into_iter().map(|r| r.map(|payload| payload)), // echo each frame
        ));

        Ok((out, GrpcMetadata::default()))
    })
}
```

### 3. Wire with TonicGrpcServer

```rust
use std::sync::Arc;
use swe_edge_ingress_grpc::TonicGrpcServer;

let handler = Arc::new(MyHandler);
let server  = TonicGrpcServer::new("0.0.0.0:50051", handler);

// With a Ctrl+C shutdown signal
server.serve(async {
    let _ = tokio::signal::ctrl_c().await;
}).await?;

// With a message size override (default is MAX_MESSAGE_BYTES = 4 MiB)
let server = TonicGrpcServer::new("0.0.0.0:50051", handler)
    .with_max_message_size(8 * 1024 * 1024); // 8 MiB
```

**No proto-gen required.** Bytes flow to `handle_unary`/`handle_stream` with gRPC length-prefix framing stripped. Parse and serialize protos manually or with `prost::Message`'s encode/decode methods.

**Error → gRPC status code mapping** (handled automatically by `TonicGrpcServer`):

| `GrpcInboundError` variant | gRPC status code |
|---|---|
| `NotFound` | 5 |
| `InvalidArgument` | 3 |
| `DeadlineExceeded` | 4 |
| `PermissionDenied` | 7 |
| `Unimplemented` | 12 |
| `Unavailable` | 14 |
| `Internal` | 13 |

---

## Using FileInbound

`FileInbound` is a read-only file source. Use it for config loading, static asset serving, or any inbound file read path.

```rust
use swe_edge_ingress_file::{FileInbound, ListOptions, local_file_source};

// All paths are resolved relative to base_path.
let source = local_file_source("/var/app/data");

// Read a file
let bytes = source.read("config/settings.toml").await?;

// Check existence
let exists = source.exists("uploads/photo.jpg").await?;

// File metadata
let info = source.metadata("uploads/photo.jpg").await?;
// info.size, info.modified, info.name

// List a directory
let result = source.list(ListOptions::default()).await?;

// Streaming list (lazy; no extra round-trips)
let mut stream = source.list_stream(ListOptions::default()).await?;
while let Some(item) = stream.next().await {
    println!("{}", item?.name);
}

// Presigned read URL (useful when delegating downloads to clients)
let url = source.presigned_read_url("uploads/photo.jpg", 3600).await?;
// url.url — String
// url.expires_at — DateTime<Utc>
```

---

## Wiring with DaemonRunner

`DaemonRunner` bootstraps a gateway process. It initialises structured logging via `swe_justobserv_context::LogContext` and optionally spawns the `ObsrvProcess` observability sidecar. It does not start any server — the caller's closure does that.

```rust
use swe_edge_ingress::{DaemonRunner, DaemonContext};

DaemonRunner::new("my-service")
    // Optional — defaults shown
    .with_bind("0.0.0.0:9000")
    .with_backend("sidecar")
    .with_obsrv_port(9001)
    // Skip the ObsrvProcess sidecar (useful in tests and local development)
    .without_observability()
    .run(|ctx: DaemonContext| async move {
        // ctx.daemon_id    — UUIDv4 String, unique per process start
        // ctx.bind         — SocketAddr, resolved from with_bind()
        // ctx.service_name — String
        // ctx.backend      — String
        // ctx.obsrv_port   — u16

        let handler = Arc::new(MyHandler);
        let server  = AxumHttpServer::new(ctx.bind.to_string(), handler);
        server.serve(async {
            let _ = tokio::signal::ctrl_c().await;
        }).await?;

        Ok(())
    })
    .await?;
```

If the `ObsrvProcess` sidecar binary is absent from `$PATH`, `DaemonRunner` silently falls back to in-memory observability — no crash, no configuration required during development.

---

## Writing tests

### Unit tests

Tests with no I/O live inline in `#[cfg(test)]` modules. Name them `test_<action>_<condition>_<expectation>`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_unary_with_empty_body_returns_ok() {
        let h   = MyHandler;
        let req = GrpcRequest { method: "/svc/Op".into(), body: vec![], metadata: GrpcMetadata::default() };
        let res = h.handle_unary(req).await.unwrap();
        assert!(res.body.is_empty());
    }
}
```

Test-only helper methods belong in a `#[cfg(test)] impl Type { ... }` block to avoid `dead_code` warnings under `cargo clippy -D warnings`.

### Integration tests — HTTP

Integration tests live in `tests/*.rs` and are named `*_int_test.rs`. They bind a real TCP listener on port 0 and exercise the full server path.

```rust
// tests/my_handler_int_test.rs

use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use swe_edge_ingress_http::{AxumHttpServer, HttpInbound};

async fn start_server(handler: Arc<dyn HttpInbound>) -> (String, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel::<()>();

    let server = AxumHttpServer::new(addr.to_string(), handler);
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });

    (format!("http://{addr}"), tx)
}

#[tokio::test]
async fn test_get_health_returns_200() {
    let (base, _shutdown) = start_server(Arc::new(MyHandler)).await;
    let resp = reqwest::get(format!("{base}/__health")).await.unwrap();
    assert_eq!(resp.status(), 200);
}
```

Key points:
- Bind on port 0 — the OS assigns an ephemeral port; no conflicts between test threads.
- Hold `_shutdown` until the test ends — dropping the sender signals the server to stop.
- Add `reqwest` to `[dev-dependencies]` for HTTP assertions.

### Integration tests — gRPC

gRPC tests require raw HTTP/2 — use `hyper::client::conn::http2` directly. See `grpc/tests/tonic_grpc_server_int_test.rs` for the full helper set (`start_server`, `grpc_call`, `grpc_frames_body`, `parse_grpc_frames`).

```rust
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use swe_edge_ingress_grpc::{GrpcInbound, TonicGrpcServer};

async fn start_server<H: GrpcInbound + 'static>(handler: H) -> (std::net::SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let server   = TonicGrpcServer::new("127.0.0.1:0", Arc::new(handler));
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(10)).await; // wait for bind
    (addr, tx)
}
```

gRPC length-prefix framing (5-byte header: 1 compressed-flag byte + 4-byte big-endian length):

```rust
fn grpc_frame(payload: &[u8]) -> bytes::Bytes {
    use bytes::{BufMut, BytesMut};
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0); // not compressed
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}
```

### FileInbound tests

Use a `tempfile::TempDir` as the base path:

```rust
use tempfile::TempDir;
use swe_edge_ingress_file::local_file_source;

#[tokio::test]
async fn test_read_returns_written_bytes() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("hello.txt"), b"world").unwrap();

    let src = local_file_source(dir.path().to_str().unwrap());
    let bytes = src.read("hello.txt").await.unwrap();
    assert_eq!(bytes, b"world");
}
```

---

## Request value objects

### HttpRequest builders

```rust
use swe_edge_ingress_http::{HttpBody, HttpMethod, HttpRequest};

// Convenience constructors
let req = HttpRequest::get("/resource");
let req = HttpRequest::post("/items");

// With a JSON body
let req = HttpRequest::post("/items")
    .with_json(&serde_json::json!({"name": "widget"}))?;

// With a raw body and custom header
let req = HttpRequest {
    method:  HttpMethod::Put,
    url:     "/config".into(),
    headers: [("content-type".into(), "text/plain".into())].into(),
    query:   Default::default(),
    body:    Some(HttpBody::Raw(b"raw bytes".to_vec())),
    auth:    None,
    timeout: Some(std::time::Duration::from_secs(10)),
};
```

### HttpResponse

```rust
use swe_edge_ingress_http::HttpResponse;

let resp = HttpResponse::new(200, body_bytes);
assert!(resp.is_success());       // 2xx
assert!(!resp.is_client_error()); // 4xx
assert!(!resp.is_server_error()); // 5xx

// Parse body as text
let text = resp.text().unwrap_or_default();
```

---

## Implementing a new inbound trait

All domain traits follow the same pattern. Example: adding a new `QueueInbound` crate.

**1. Define the trait in `api/port/`:**

```rust
// queue/src/api/port/queue_inbound.rs
pub trait QueueInbound: Send + Sync {
    fn consume(&self, msg: QueueMessage) -> BoxFuture<'_, QueueInboundResult<QueueAck>>;
    fn health_check(&self) -> BoxFuture<'_, QueueInboundResult<QueueHealthCheck>>;
}
```

**2. Implement in `core/`:**

```rust
// queue/src/core/default_queue.rs
pub(crate) struct DefaultQueueConsumer { ... }

impl QueueInbound for DefaultQueueConsumer { ... }
```

**3. Expose via `saf/`:**

```rust
// queue/src/saf/mod.rs
use crate::core::DefaultQueueConsumer;
pub use crate::api::port::{QueueInbound, QueueInboundResult, QueueInboundError};
pub use crate::api::value_object::{QueueMessage, QueueAck, QueueHealthCheck};

pub fn queue_consumer(config: QueueConfig) -> Result<impl QueueInbound, Error> {
    DefaultQueueConsumer::new(config)
}
```

**4. Re-export from `lib.rs`:**

```rust
mod api; mod core; mod saf;
pub use saf::*;
```

Consumers get `impl QueueInbound` and never see `DefaultQueueConsumer`.

---

## Debugging

**Inspect what the server received:**

Add a `println!` or `tracing::debug!` in `handle` before the match — `AxumHttpServer` passes the raw `HttpRequest` including all extracted fields.

**Trace gRPC framing issues:**

Enable tracing in tests with `tracing_subscriber::fmt::init()` and check for `swe_edge_ingress_grpc` log lines. The server logs each dispatch at `DEBUG` level.

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
