//! Integration tests for [`GrpcServerConfig`] and the
//! [`TonicGrpcServer::from_config`] fail-closed gate.
//!
//! These verify two of issue #5's headline acceptance criteria:
//!
//!   1. `GrpcServerConfig::default()` advertises `tls_required = true`
//!   2. A plaintext connection to a TLS-required server fails BEFORE
//!      the handler runs

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use http::Request;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use swe_edge_ingress_grpc::{
    GrpcHealthCheck, GrpcInbound, GrpcInboundResult, GrpcMetadata, GrpcRequest, GrpcResponse,
    GrpcServerConfig, GrpcServerConfigError, IngressTlsConfig, TonicGrpcServer,
};

// ── Stub handler with hit-recording ─────────────────────────────────────────

struct RecordingHandler {
    hit: Arc<AtomicBool>,
}

impl GrpcInbound for RecordingHandler {
    fn handle_unary(
        &self,
        req: GrpcRequest,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        let hit = self.hit.clone();
        Box::pin(async move {
            hit.store(true, Ordering::SeqCst);
            Ok(GrpcResponse { body: req.body, metadata: GrpcMetadata::default() })
        })
    }

    fn health_check(
        &self,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async { Ok(GrpcHealthCheck::healthy()) })
    }
}

fn self_signed() -> (String, String) {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    (cert.cert.pem(), cert.key_pair.serialize_pem())
}

fn write_temp(content: &str) -> tempfile::NamedTempFile {
    use std::io::Write as _;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

fn grpc_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

// ── Headline acceptance gates ───────────────────────────────────────────────

/// @covers: GrpcServerConfig::default — `tls_required` defaults to true.
/// Issue #5 acceptance gate (1/4).
#[test]
fn grpc_server_config_struct_default_requires_tls_int_test() {
    let cfg = GrpcServerConfig::default();
    assert!(cfg.tls_required, "TLS-by-default invariant must hold");
}

/// @covers: TonicGrpcServer::from_config — rejects when tls_required is set
/// but no IngressTlsConfig is supplied.
#[test]
fn tonic_grpc_grpc_server_struct_from_config_rejects_tls_required_without_tls_int_test() {
    let cfg = GrpcServerConfig::default(); // tls_required=true, tls=None
    let handler = Arc::new(RecordingHandler { hit: Arc::new(AtomicBool::new(false)) });
    match TonicGrpcServer::from_config(&cfg, handler) {
        Err(GrpcServerConfigError::TlsRequiredButMissing) => {}
        Ok(_) => panic!("must reject tls_required=true with tls=None"),
    }
}

/// @covers: TonicGrpcServer::from_config — accepts when allow_plaintext is set.
#[test]
fn tonic_grpc_server_struct_from_config_accepts_plaintext_with_opt_in_int_test() {
    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let cfg = GrpcServerConfig::new(bind).allow_plaintext();
    let handler = Arc::new(RecordingHandler { hit: Arc::new(AtomicBool::new(false)) });
    assert!(TonicGrpcServer::from_config(&cfg, handler).is_ok());
}

/// @covers: TonicGrpcServer::from_config — accepts when TLS is supplied.
#[test]
fn tonic_grpc_server_struct_from_config_accepts_tls_int_test() {
    let (cert_pem, key_pem) = self_signed();
    let cert_f = write_temp(&cert_pem);
    let key_f  = write_temp(&key_pem);
    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let cfg = GrpcServerConfig::new(bind).with_tls(IngressTlsConfig::tls(
        cert_f.path().to_str().unwrap(),
        key_f.path().to_str().unwrap(),
    ));
    let handler = Arc::new(RecordingHandler { hit: Arc::new(AtomicBool::new(false)) });
    assert!(TonicGrpcServer::from_config(&cfg, handler).is_ok());
}

/// @covers: end-to-end gate — a plaintext h2c connection to a TLS-required
/// server fails BEFORE the handler runs.  Issue #5 acceptance gate (2/4).
///
/// The server is configured with TLS material (so it accepts on the
/// listener) and the handler stores `hit=true` if it ever runs.  We
/// connect over plain h2c (no TLS) and assert the handshake fails or
/// the request fails at the transport layer — and crucially that
/// `hit` remains `false`.
#[tokio::test]
async fn plaintext_to_tls_required_server_fails_before_handler_runs_int_test() {
    let (cert_pem, key_pem) = self_signed();
    let cert_f = write_temp(&cert_pem);
    let key_f  = write_temp(&key_pem);
    let tls_cfg = IngressTlsConfig::tls(
        cert_f.path().to_str().unwrap(),
        key_f.path().to_str().unwrap(),
    );

    let hit = Arc::new(AtomicBool::new(false));
    let handler = Arc::new(RecordingHandler { hit: hit.clone() });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let server = TonicGrpcServer::new("127.0.0.1:0", handler).with_tls(tls_cfg);
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect with PLAIN h2c — no TLS handshake.  The TLS server will
    // see TLS-record-shaped expectations; an h2c preface is invalid TLS
    // so the connection MUST fail.
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io     = TokioIo::new(stream);
    let handshake_result = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake::<_, Full<Bytes>>(io)
        .await;

    let attempted = match handshake_result {
        Ok((mut sender, conn)) => {
            tokio::spawn(conn);
            let req = Request::builder()
                .method("POST")
                .uri(format!("http://{addr}/pkg.Svc/Echo"))
                .header("content-type", "application/grpc")
                .header("te", "trailers")
                .body(Full::new(grpc_frame(b"plain-attack")))
                .unwrap();
            sender.send_request(req).await
        }
        Err(_) => {
            // h2c handshake itself failed — that is the expected failure mode.
            let _ = tx.send(());
            assert!(!hit.load(Ordering::SeqCst), "handler ran despite TLS-required");
            return;
        }
    };

    // Either the request failed outright, or it returned a non-grpc-OK
    // response.  Either way, the handler MUST NOT have run.
    let resp_ok = matches!(&attempted, Ok(_));
    if resp_ok {
        // Drain the body — even when the server returns a response,
        // the handler should not have executed.
        if let Ok(resp) = attempted {
            let _ = resp.into_body().collect().await;
        }
    }

    assert!(
        !hit.load(Ordering::SeqCst),
        "handler MUST NOT run when plaintext client hits TLS-required server",
    );

    let _ = tx.send(());
}

/// @covers: TonicGrpcServer::with_compression — server attaches
/// `grpc-accept-encoding` to response trailers when set.
#[tokio::test]
async fn tonic_grpc_server_struct_advertises_grpc_accept_encoding_when_gzip_set_int_test() {
    use swe_edge_ingress_grpc::CompressionMode;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let hit      = Arc::new(AtomicBool::new(false));
    let server   = TonicGrpcServer::new("127.0.0.1:0", Arc::new(RecordingHandler { hit: hit.clone() }))
        .with_compression(CompressionMode::Gzip);
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(20)).await;

    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io     = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(io)
        .await
        .unwrap();
    tokio::spawn(conn);

    let req = Request::builder()
        .method("POST")
        .uri(format!("http://{addr}/pkg.Svc/Echo"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(Full::new(grpc_frame(b"hi")))
        .unwrap();
    let resp = sender.send_request(req).await.unwrap();
    let collected = resp.into_body().collect().await.unwrap();
    let trailers = collected.trailers().expect("trailers must be present");
    let advertised = trailers
        .get("grpc-accept-encoding")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    assert_eq!(advertised.as_deref(), Some("gzip"));
    assert!(hit.load(Ordering::SeqCst), "handler must have run for compressed call");

    let _ = tx.send(());
}
