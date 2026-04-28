//! Phase 3 acceptance integration tests for the default-deny
//! authorisation invariant and the [`AuditSink`] callback.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use futures::future::BoxFuture;
use http::Request;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use swe_edge_ingress_grpc::{
    AuditEvent, AuditSink, AuthorizationInterceptor, GrpcHealthCheck, GrpcInbound,
    GrpcInboundError, GrpcInboundInterceptor, GrpcInboundInterceptorChain,
    GrpcInboundResult, GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode,
    TonicGrpcServer,
};

// ── Test fixtures ─────────────────────────────────────────────────────────────

struct EchoHandler;
impl GrpcInbound for EchoHandler {
    fn handle_unary(&self, req: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            Ok(GrpcResponse { body: req.body, metadata: GrpcMetadata::default() })
        })
    }
    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async { Ok(GrpcHealthCheck::healthy()) })
    }
}

/// Authz interceptor that always denies, but with a *detailed* error
/// message — used to verify the wire never sees the rationale.
struct DetailedDenyInterceptor;
impl GrpcInboundInterceptor for DetailedDenyInterceptor {
    fn before_dispatch(&self, _: &mut GrpcRequest) -> Result<(), GrpcInboundError> {
        Err(GrpcInboundError::Status(
            GrpcStatusCode::PermissionDenied,
            "policy decision: subject=mallory@evil.example role=anonymous \
             rejected by ROLE_ADMIN (rule_id=42)".into(),
        ))
    }
    fn after_dispatch(&self, _: &mut GrpcResponse) -> Result<(), GrpcInboundError> { Ok(()) }
    fn is_authorization(&self) -> bool { true }
}
impl AuthorizationInterceptor for DetailedDenyInterceptor {}

/// Authz interceptor that always allows.
struct AllowAllInterceptor;
impl GrpcInboundInterceptor for AllowAllInterceptor {
    fn before_dispatch(&self, _: &mut GrpcRequest) -> Result<(), GrpcInboundError> { Ok(()) }
    fn after_dispatch(&self, _: &mut GrpcResponse) -> Result<(), GrpcInboundError> { Ok(()) }
    fn is_authorization(&self) -> bool { true }
}
impl AuthorizationInterceptor for AllowAllInterceptor {}

struct CapturingAuditSink {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}
impl AuditSink for CapturingAuditSink {
    fn record(&self, event: AuditEvent) {
        self.events.lock().unwrap().push(event);
    }
}

fn grpc_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

async fn start_server(
    handler: Arc<dyn GrpcInbound>,
    chain:   GrpcInboundInterceptorChain,
    sink:    Arc<dyn AuditSink>,
    allow_unauthenticated: bool,
) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let server   = TonicGrpcServer::new("127.0.0.1:0", handler)
        .with_interceptors(chain)
        .with_audit_sink(sink)
        .allow_unauthenticated(allow_unauthenticated);
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    (addr, tx)
}

async fn grpc_call(
    addr: SocketAddr,
    path: &str,
    payload: &[u8],
) -> (Option<String>, Option<String>) {
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
    let trailers = collected.trailers();
    let grpc_status = trailers
        .and_then(|t| t.get("grpc-status"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let grpc_message = trailers
        .and_then(|t| t.get("grpc-message"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    (grpc_status, grpc_message)
}

// ── Phase 3 default-deny acceptance gates ─────────────────────────────────────

/// @covers: TonicGrpcServer — server with no authz + fail-closed default panics.
///
/// Issue #6 acceptance gate.
#[tokio::test]
#[should_panic(expected = "AuthorizationInterceptor")]
async fn test_server_panics_at_startup_when_no_authz_and_fail_closed_default() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server   = TonicGrpcServer::new("127.0.0.1:0", Arc::new(EchoHandler));
    // Default allow_unauthenticated = false; no authz interceptor in the
    // empty chain → must panic before bind.
    let _ = server
        .serve_with_listener(listener, std::future::pending::<()>())
        .await;
}

/// @covers: TonicGrpcServer — `allow_unauthenticated = true` lets the server
/// bind without an authz interceptor.
#[tokio::test]
async fn test_server_starts_when_allow_unauthenticated_is_true_and_no_authz() {
    let (_, _shutdown) = start_server(
        Arc::new(EchoHandler),
        GrpcInboundInterceptorChain::new(),
        Arc::new(swe_edge_ingress_grpc::NoopAuditSink),
        true, // allow_unauthenticated
    ).await;
    // Reaching this point means the server bound successfully — the
    // panic in `enforce_authorization_invariant` was suppressed by
    // the WARN-and-proceed branch.
}

// ── PermissionDenied on-wire sanitisation ─────────────────────────────────────

/// @covers: dispatch — PermissionDenied authz error never leaks the
/// policy-decision rationale to the client.
///
/// Issue #6 acceptance gate.
#[tokio::test]
async fn test_authz_permission_denied_sanitizes_policy_rationale_on_the_wire() {
    let chain = GrpcInboundInterceptorChain::new()
        .push(Arc::new(DetailedDenyInterceptor));
    let (addr, _shutdown) = start_server(
        Arc::new(EchoHandler),
        chain,
        Arc::new(swe_edge_ingress_grpc::NoopAuditSink),
        false, // authz IS present
    ).await;

    let (grpc_status, grpc_message) = grpc_call(addr, "/pkg.Svc/M", b"x").await;
    // tonic::Code::PermissionDenied == 7
    assert_eq!(grpc_status.as_deref(), Some("7"));

    let msg = grpc_message.unwrap_or_default();
    assert!(
        !msg.contains("mallory")
            && !msg.contains("ROLE_ADMIN")
            && !msg.contains("rule_id")
            && !msg.contains("subject"),
        "wire grpc-message leaked policy decision rationale: {msg}"
    );
    assert_eq!(
        msg, "authorization denied",
        "wire message must be the canonical sanitized string"
    );
}

// ── AuditSink — every dispatch records exactly one event ──────────────────────

/// @covers: TonicGrpcServer + AuditSink — every dispatch fires `AuditSink::record`.
///
/// Issue #6 acceptance gate.
#[tokio::test]
async fn test_audit_sink_records_one_event_per_authenticated_dispatch() {
    let events = Arc::new(Mutex::new(Vec::<AuditEvent>::new()));
    let sink: Arc<dyn AuditSink> = Arc::new(CapturingAuditSink { events: events.clone() });
    let chain = GrpcInboundInterceptorChain::new().push(Arc::new(AllowAllInterceptor));

    let (addr, _shutdown) = start_server(
        Arc::new(EchoHandler),
        chain,
        sink,
        false,
    ).await;

    let _ = grpc_call(addr, "/pkg.Svc/Echo", b"first").await;
    let _ = grpc_call(addr, "/pkg.Svc/Echo", b"second").await;
    let _ = grpc_call(addr, "/pkg.Svc/Echo", b"third").await;

    // Allow the server side a moment to flush trailing audit events.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let captured = events.lock().unwrap().clone();
    assert_eq!(
        captured.len(), 3,
        "expected exactly 3 audit events, got {}",
        captured.len(),
    );
    for evt in &captured {
        assert_eq!(evt.method, "/pkg.Svc/Echo");
        assert_eq!(evt.status, GrpcStatusCode::Ok);
    }
}

/// @covers: TonicGrpcServer + AuditSink — authz-rejected requests still
/// record an audit event with the PermissionDenied status.
#[tokio::test]
async fn test_audit_sink_records_event_for_authz_rejected_dispatch() {
    let events = Arc::new(Mutex::new(Vec::<AuditEvent>::new()));
    let sink: Arc<dyn AuditSink> = Arc::new(CapturingAuditSink { events: events.clone() });
    let chain = GrpcInboundInterceptorChain::new().push(Arc::new(DetailedDenyInterceptor));

    let (addr, _shutdown) = start_server(
        Arc::new(EchoHandler),
        chain,
        sink,
        false,
    ).await;

    let (grpc_status, _) = grpc_call(addr, "/pkg.Svc/Drop", b"x").await;
    assert_eq!(grpc_status.as_deref(), Some("7"));

    tokio::time::sleep(Duration::from_millis(30)).await;
    let captured = events.lock().unwrap().clone();
    assert_eq!(captured.len(), 1, "expected exactly 1 audit event");
    assert_eq!(captured[0].method, "/pkg.Svc/Drop");
    assert_eq!(captured[0].status, GrpcStatusCode::PermissionDenied);
}
