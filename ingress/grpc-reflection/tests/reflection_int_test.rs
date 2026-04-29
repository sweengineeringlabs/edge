//! Phase 5 acceptance integration tests for the reflection crate.
//!
//! These prove the grpcurl-equivalent wire path:
//!
//! 1. Start a `TonicGrpcServer` with `enable_reflection = true`,
//!    register a stub handler under `/pkg.Demo/Echo`, register the
//!    `ReflectionService` under `REFLECTION_INFO_METHOD`, and verify
//!    that `ListServices` returns both `pkg.Demo` and the reflection
//!    self-name.
//!
//! 2. Start a server WITHOUT registering the reflection service —
//!    the dispatcher answers `Unimplemented (12)` on the wire,
//!    matching the default-off acceptance criterion.
//!
//! 3. Start a server with `enable_reflection = true` and confirm a
//!    `tracing` WARN with the expected message fires before serving.
//!
//! 4. With a `FileDescriptorSet` registered, `FileByFilename` and
//!    `FileContainingSymbol` return the registered bytes; without
//!    one they return a structured `ErrorResponse(NOT_FOUND)`.

use std::any::Any;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use http::Request;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use edge_domain::{Handler, HandlerError, HandlerRegistry};
use swe_edge_ingress_grpc::{
    GrpcHandlerAdapter, GrpcInboundError, HandlerRegistryDispatcher, TonicGrpcServer,
    REFLECTION_ENABLED_WARN_MSG,
};
use swe_edge_ingress_grpc_reflection::{
    Descriptor, ReflectionService, REFLECTION_INFO_METHOD, REFLECTION_SERVICE_NAME,
};

// ── stub handler — proves a normal service shows up in ListServices ───────────

#[derive(Debug, PartialEq, Eq)]
struct EchoReq(Vec<u8>);

#[derive(Debug, PartialEq, Eq)]
struct EchoResp(Vec<u8>);

fn decode_echo(bytes: &[u8]) -> Result<EchoReq, GrpcInboundError> {
    Ok(EchoReq(bytes.to_vec()))
}

fn encode_echo(resp: &EchoResp) -> Vec<u8> { resp.0.clone() }

struct EchoHandler;

#[async_trait]
impl Handler<EchoReq, EchoResp> for EchoHandler {
    fn id(&self) -> &str { "/pkg.Demo/Echo" }
    fn pattern(&self) -> &str { "demo" }
    async fn execute(&self, req: EchoReq) -> Result<EchoResp, HandlerError> {
        Ok(EchoResp(req.0))
    }
    async fn health_check(&self) -> bool { true }
    fn as_any(&self) -> &dyn Any { self }
}

// ── reflection adapter — turns ReflectionService into a registered handler ────
//
// We can't use GrpcHandlerAdapter directly because the reflection
// service needs the raw ServerReflectionRequest bytes (the codec is
// inside ReflectionService::handle_unary).  Instead we register a
// thin wrapper that forwards Vec<u8> -> Vec<u8> through the service.

struct ReflectionHandlerWrapper {
    service: Arc<ReflectionService>,
}

#[async_trait]
impl Handler<Vec<u8>, Vec<u8>> for ReflectionHandlerWrapper {
    fn id(&self) -> &str { REFLECTION_INFO_METHOD }
    fn pattern(&self) -> &str { "reflection" }
    async fn execute(&self, req: Vec<u8>) -> Result<Vec<u8>, HandlerError> {
        use swe_edge_ingress_grpc::GrpcInbound;
        use swe_edge_ingress_grpc::GrpcRequest;
        use std::time::Duration;
        let r = GrpcRequest::new(REFLECTION_INFO_METHOD, req, Duration::from_secs(5));
        match self.service.handle_unary(r).await {
            Ok(resp) => Ok(resp.body),
            Err(e) => Err(HandlerError::ExecutionFailed(e.to_string())),
        }
    }
    async fn health_check(&self) -> bool { true }
    fn as_any(&self) -> &dyn Any { self }
}

// ── wire helpers ──────────────────────────────────────────────────────────────

fn grpc_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

fn parse_first_grpc_frame(data: &Bytes) -> Option<Bytes> {
    if data.len() < 5 {
        return None;
    }
    let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
    if data.len() < 5 + len {
        return None;
    }
    Some(data.slice(5..5 + len))
}

fn encode_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn decode_varint(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut result = 0u64;
    let mut shift  = 0u32;
    for (i, byte) in bytes.iter().take(10).enumerate() {
        result |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift += 7;
    }
    None
}

/// Build a `ServerReflectionRequest { list_services: "" }` body.
fn build_list_services_request() -> Vec<u8> {
    // tag 7 wire 2 = 0x3a, length 0
    vec![0x3a, 0x00]
}

/// Build a `ServerReflectionRequest { file_by_filename: name }` body.
fn build_file_by_filename_request(name: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0x1a); // tag 3, wire 2
    encode_varint(name.len() as u64, &mut out);
    out.extend_from_slice(name.as_bytes());
    out
}

/// Build a `ServerReflectionRequest { file_containing_symbol: symbol }` body.
fn build_file_containing_symbol_request(symbol: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0x22); // tag 4, wire 2
    encode_varint(symbol.len() as u64, &mut out);
    out.extend_from_slice(symbol.as_bytes());
    out
}

/// Decode the names list from a `ListServiceResponse` payload.
///
/// Walks the outer `ServerReflectionResponse`, finds the
/// `list_services_response = 6` sub-message, then collects every
/// `ServiceResponse.name = 1` entry inside.
fn parse_list_services_names(body: &[u8]) -> Vec<String> {
    let mut idx = 0usize;
    let mut list_payload: Option<&[u8]> = None;
    while idx < body.len() {
        let (tag, c) = decode_varint(&body[idx..]).expect("tag varint");
        idx += c;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        match (fnum, wire) {
            (_, 2) => {
                let (len, c2) = decode_varint(&body[idx..]).expect("length varint");
                idx += c2;
                let end = idx + len as usize;
                if fnum == 6 {
                    list_payload = Some(&body[idx..end]);
                }
                idx = end;
            }
            (_, 0) => {
                let (_, c2) = decode_varint(&body[idx..]).expect("varint value");
                idx += c2;
            }
            (_, wt) => panic!("unsupported wire type {wt}"),
        }
    }
    let payload = list_payload.expect("ListServiceResponse payload present");

    // payload is repeated ServiceResponse { name=1 string }.
    let mut names = Vec::new();
    let mut idx = 0;
    while idx < payload.len() {
        let (tag, c) = decode_varint(&payload[idx..]).expect("tag");
        idx += c;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        if (fnum, wire) != (1, 2) {
            // skip
            let (len, c2) = decode_varint(&payload[idx..]).expect("len");
            idx += c2;
            idx += len as usize;
            continue;
        }
        let (len, c2) = decode_varint(&payload[idx..]).expect("svc length");
        idx += c2;
        let end = idx + len as usize;
        let svc = &payload[idx..end];
        idx = end;

        // svc is ServiceResponse { name=1 string }
        let mut sidx = 0;
        while sidx < svc.len() {
            let (tag, sc) = decode_varint(&svc[sidx..]).expect("svc tag");
            sidx += sc;
            let fnum = (tag >> 3) as u32;
            let wire = (tag & 0x7) as u8;
            if (fnum, wire) == (1, 2) {
                let (len, sc2) = decode_varint(&svc[sidx..]).expect("name length");
                sidx += sc2;
                let bytes = &svc[sidx..sidx + len as usize];
                sidx += len as usize;
                names.push(std::str::from_utf8(bytes).expect("utf8").to_string());
            } else {
                let (len, sc2) = decode_varint(&svc[sidx..]).expect("len");
                sidx += sc2;
                sidx += len as usize;
            }
        }
    }
    names
}

/// Walk the response and pull the first FileDescriptorResponse.file_descriptor_proto entry, if any.
fn parse_first_file_descriptor_bytes(body: &[u8]) -> Option<Vec<u8>> {
    let mut idx = 0usize;
    while idx < body.len() {
        let (tag, c) = decode_varint(&body[idx..])?;
        idx += c;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        if wire != 2 {
            // skip varint values
            let (_, c2) = decode_varint(&body[idx..])?;
            idx += c2;
            continue;
        }
        let (len, c2) = decode_varint(&body[idx..])?;
        idx += c2;
        let end = idx + len as usize;
        if fnum == 4 {
            // file_descriptor_response — repeated bytes file_descriptor_proto = 1
            let payload = &body[idx..end];
            let mut sidx = 0;
            while sidx < payload.len() {
                let (tag, sc) = decode_varint(&payload[sidx..])?;
                sidx += sc;
                let fnum = (tag >> 3) as u32;
                let wire = (tag & 0x7) as u8;
                if (fnum, wire) == (1, 2) {
                    let (l, sc2) = decode_varint(&payload[sidx..])?;
                    sidx += sc2;
                    let bytes = payload[sidx..sidx + l as usize].to_vec();
                    return Some(bytes);
                } else if wire == 2 {
                    let (l, sc2) = decode_varint(&payload[sidx..])?;
                    sidx += sc2;
                    sidx += l as usize;
                } else {
                    let (_, sc2) = decode_varint(&payload[sidx..])?;
                    sidx += sc2;
                }
            }
        }
        idx = end;
    }
    None
}

/// Walk the response and pull `error_response.error_code` (field 7 -> field 1).
fn parse_error_response_code(body: &[u8]) -> Option<i32> {
    let mut idx = 0usize;
    while idx < body.len() {
        let (tag, c) = decode_varint(&body[idx..])?;
        idx += c;
        let fnum = (tag >> 3) as u32;
        let wire = (tag & 0x7) as u8;
        if wire != 2 {
            let (_, c2) = decode_varint(&body[idx..])?;
            idx += c2;
            continue;
        }
        let (len, c2) = decode_varint(&body[idx..])?;
        idx += c2;
        let end = idx + len as usize;
        if fnum == 7 {
            // error_response — int32 error_code = 1, string error_message = 2
            let payload = &body[idx..end];
            let mut sidx = 0;
            while sidx < payload.len() {
                let (tag, sc) = decode_varint(&payload[sidx..])?;
                sidx += sc;
                let fnum = (tag >> 3) as u32;
                let wire = (tag & 0x7) as u8;
                if (fnum, wire) == (1, 0) {
                    let (v, sc2) = decode_varint(&payload[sidx..])?;
                    sidx += sc2;
                    return Some(v as i32);
                } else if wire == 0 {
                    let (_, sc2) = decode_varint(&payload[sidx..])?;
                    sidx += sc2;
                } else {
                    let (l, sc2) = decode_varint(&payload[sidx..])?;
                    sidx += sc2;
                    sidx += l as usize;
                }
            }
        }
        idx = end;
    }
    None
}

// ── Server boot + grpc call helpers ───────────────────────────────────────────

async fn start_server_with_dispatcher(
    dispatcher: HandlerRegistryDispatcher,
    enable_reflection: bool,
) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let server   = TonicGrpcServer::new("127.0.0.1:0", Arc::new(dispatcher))
        .allow_unauthenticated(true)
        .enable_reflection(enable_reflection);
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    (addr, tx)
}

async fn grpc_call(
    addr: SocketAddr,
    path: &str,
    payload: &[u8],
) -> (Option<String>, Bytes) {
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(io)
        .await
        .unwrap();
    tokio::spawn(conn);

    let req = Request::builder()
        .method("POST")
        .uri(format!("http://{addr}{path}"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(Full::new(grpc_frame(payload)))
        .unwrap();

    let resp      = sender.send_request(req).await.unwrap();
    let collected = resp.into_body().collect().await.unwrap();
    let grpc_status = collected
        .trailers()
        .and_then(|t| t.get("grpc-status"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let data = collected.to_bytes();
    (grpc_status, data)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Build a dispatcher pre-loaded with EchoHandler under `/pkg.Demo/Echo`.
/// Returns the registry as well so the caller can pass it into ReflectionService.
fn dispatcher_with_echo() -> (HandlerRegistryDispatcher, Arc<HandlerRegistry<Vec<u8>, Vec<u8>>>) {
    let registry: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>> = Arc::new(HandlerRegistry::new());
    let dispatcher = HandlerRegistryDispatcher::new(registry.clone());
    dispatcher.register(GrpcHandlerAdapter::new(
        Arc::new(EchoHandler),
        decode_echo,
        encode_echo,
    ));
    (dispatcher, registry)
}

/// @covers: Phase 5 acceptance — server with `enable_reflection = true` exposes ListServices,
/// returning both the registered Echo service AND the reflection self-name.
#[tokio::test]
async fn test_list_services_returns_registered_handlers_when_reflection_enabled() {
    let (dispatcher, registry) = dispatcher_with_echo();
    // Register the reflection wrapper under REFLECTION_INFO_METHOD.
    let svc = Arc::new(ReflectionService::new(registry));
    registry_register_reflection(&dispatcher, svc);

    let (addr, _shutdown) = start_server_with_dispatcher(dispatcher, true).await;
    let body = build_list_services_request();
    let (status, data) = grpc_call(addr, REFLECTION_INFO_METHOD, &body).await;
    assert_eq!(status.as_deref(), Some("0"), "expected grpc-status=0");

    let payload = parse_first_grpc_frame(&data).expect("response frame present");
    let names = parse_list_services_names(&payload);
    assert!(
        names.iter().any(|n| n == "pkg.Demo"),
        "ListServices must include pkg.Demo: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == REFLECTION_SERVICE_NAME),
        "ListServices must include reflection self-name: {names:?}"
    );
}

/// Helper used by the tests above — registers the reflection wrapper under
/// REFLECTION_INFO_METHOD on the dispatcher.
fn registry_register_reflection(
    dispatcher: &HandlerRegistryDispatcher,
    service: Arc<ReflectionService>,
) {
    let wrapper = ReflectionHandlerWrapper { service };
    // The dispatcher's registry takes Arc<dyn Handler<Vec<u8>, Vec<u8>>>.
    dispatcher.registry().register(Arc::new(wrapper));
}

/// @covers: Phase 5 acceptance — `enable_reflection = false` (default) means the
/// reflection RPC is NOT registered.  grpcurl-equivalent ListServices returns
/// `Unimplemented (12)`.
#[tokio::test]
async fn test_list_services_returns_unimplemented_when_reflection_not_registered() {
    // Default-off path — we DO NOT register the reflection wrapper.
    let (dispatcher, _registry) = dispatcher_with_echo();
    let (addr, _shutdown) = start_server_with_dispatcher(dispatcher, false).await;
    let body = build_list_services_request();
    let (status, _) = grpc_call(addr, REFLECTION_INFO_METHOD, &body).await;
    // tonic::Code::Unimplemented == 12
    assert_eq!(status.as_deref(), Some("12"));
}

/// @covers: Phase 5 acceptance — `FileByFilename` returns the registered descriptor bytes.
#[tokio::test]
async fn test_file_by_filename_returns_registered_descriptor_bytes() {
    let (dispatcher, registry) = dispatcher_with_echo();
    let svc = Arc::new(
        ReflectionService::new(registry).add_descriptor(Descriptor {
            filename: "demo.proto".into(),
            symbols:  vec!["pkg.Demo".into(), "pkg.Demo.Echo".into()],
            bytes:    vec![0xde, 0xad, 0xbe, 0xef],
        }),
    );
    registry_register_reflection(&dispatcher, svc);

    let (addr, _shutdown) = start_server_with_dispatcher(dispatcher, true).await;
    let body = build_file_by_filename_request("demo.proto");
    let (status, data) = grpc_call(addr, REFLECTION_INFO_METHOD, &body).await;
    assert_eq!(status.as_deref(), Some("0"));
    let payload = parse_first_grpc_frame(&data).expect("response frame");
    let descriptor = parse_first_file_descriptor_bytes(&payload).expect("descriptor present");
    assert_eq!(descriptor, vec![0xde, 0xad, 0xbe, 0xef]);
}

/// @covers: Phase 5 acceptance — `FileByFilename` for unregistered filename yields ErrorResponse(NOT_FOUND).
#[tokio::test]
async fn test_file_by_filename_yields_not_found_when_no_descriptor_registered() {
    let (dispatcher, registry) = dispatcher_with_echo();
    let svc = Arc::new(ReflectionService::new(registry));
    registry_register_reflection(&dispatcher, svc);

    let (addr, _shutdown) = start_server_with_dispatcher(dispatcher, true).await;
    let body = build_file_by_filename_request("missing.proto");
    let (status, data) = grpc_call(addr, REFLECTION_INFO_METHOD, &body).await;
    // Wire-level status is OK — the structured error is in the body.
    assert_eq!(status.as_deref(), Some("0"));
    let payload = parse_first_grpc_frame(&data).expect("response frame");
    let code = parse_error_response_code(&payload).expect("error_code present");
    assert_eq!(code, 5, "NOT_FOUND == 5");
}

/// @covers: Phase 5 acceptance — `FileContainingSymbol` resolves via the symbol list.
#[tokio::test]
async fn test_file_containing_symbol_locates_descriptor_via_symbol_list() {
    let (dispatcher, registry) = dispatcher_with_echo();
    let svc = Arc::new(
        ReflectionService::new(registry).add_descriptor(Descriptor {
            filename: "demo.proto".into(),
            symbols:  vec!["pkg.Demo".into(), "pkg.Demo.Echo".into()],
            bytes:    vec![1, 2, 3, 4],
        }),
    );
    registry_register_reflection(&dispatcher, svc);

    let (addr, _shutdown) = start_server_with_dispatcher(dispatcher, true).await;
    let body = build_file_containing_symbol_request("pkg.Demo.Echo");
    let (status, data) = grpc_call(addr, REFLECTION_INFO_METHOD, &body).await;
    assert_eq!(status.as_deref(), Some("0"));
    let payload = parse_first_grpc_frame(&data).expect("response frame");
    let descriptor = parse_first_file_descriptor_bytes(&payload).expect("descriptor present");
    assert_eq!(descriptor, vec![1, 2, 3, 4]);
}

// ── tracing-warn capture ──────────────────────────────────────────────────────
//
// Hooks a `tracing::subscriber` for the duration of the test, captures
// every event, and asserts the WARN string fires when a reflection-enabled
// server starts up.

struct CapturingSubscriber {
    events: Arc<Mutex<Vec<String>>>,
}

impl tracing::Subscriber for CapturingSubscriber {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.level() <= &tracing::Level::WARN
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        struct StringVisitor<'a> { acc: &'a mut String }
        impl<'a> tracing::field::Visit for StringVisitor<'a> {
            fn record_debug(&mut self, _: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                let _ = write!(self.acc, "{value:?}");
            }
            fn record_str(&mut self, _: &tracing::field::Field, value: &str) {
                use std::fmt::Write;
                let _ = write!(self.acc, "{value}");
            }
        }
        let mut s = String::new();
        let mut v = StringVisitor { acc: &mut s };
        event.record(&mut v);
        self.events.lock().unwrap().push(s);
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

/// @covers: Phase 5 acceptance — server with `enable_reflection = true` emits
/// the WARN at startup.
#[tokio::test]
async fn test_server_logs_warn_on_startup_when_reflection_enabled() {
    let events = Arc::new(Mutex::new(Vec::<String>::new()));
    let sub = CapturingSubscriber { events: events.clone() };
    let dispatch = tracing::Dispatch::new(sub);

    // Enter the dispatcher scope explicitly so the server's tokio
    // tasks pick it up.
    let _guard = tracing::dispatcher::set_default(&dispatch);

    let (dispatcher, _) = dispatcher_with_echo();
    let (_addr, _shutdown) = start_server_with_dispatcher(dispatcher, true).await;

    // Give the server a moment to log.
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;

    let captured = events.lock().unwrap();
    assert!(
        captured.iter().any(|e| e.contains(REFLECTION_ENABLED_WARN_MSG)),
        "expected WARN containing {REFLECTION_ENABLED_WARN_MSG:?}, got events: {captured:?}"
    );
}

/// @covers: Phase 5 acceptance — server with `enable_reflection = false` does
/// NOT emit the WARN.
#[tokio::test]
async fn test_server_does_not_log_reflection_warn_when_flag_disabled() {
    let events = Arc::new(Mutex::new(Vec::<String>::new()));
    let sub = CapturingSubscriber { events: events.clone() };
    let dispatch = tracing::Dispatch::new(sub);

    let _guard = tracing::dispatcher::set_default(&dispatch);

    let (dispatcher, _) = dispatcher_with_echo();
    let (_addr, _shutdown) = start_server_with_dispatcher(dispatcher, false).await;

    tokio::time::sleep(std::time::Duration::from_millis(60)).await;

    let captured = events.lock().unwrap();
    assert!(
        !captured.iter().any(|e| e.contains(REFLECTION_ENABLED_WARN_MSG)),
        "WARN must NOT fire when enable_reflection=false; events: {captured:?}"
    );
}
