//! Integration tests for TonicGrpcServer TLS.
//!
//! Each test generates a self-signed cert, starts a gRPC-over-TLS server on an
//! ephemeral port, then connects with a raw hyper HTTP/2 client backed by a
//! tokio-rustls connector that skips certificate verification.

use std::io::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::{BufMut, Bytes, BytesMut};
use http::Request;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use swe_edge_ingress_grpc::{
    GrpcHealthCheck, GrpcInbound, GrpcInboundResult, GrpcMetadata, GrpcRequest, GrpcResponse,
    IngressTlsConfig, TonicGrpcServer,
};

// ── Stub handler ─────────────────────────────────────────────────────────────

struct EchoHandler;

impl GrpcInbound for EchoHandler {
    fn handle_unary(
        &self,
        req: GrpcRequest,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            Ok(GrpcResponse { body: req.body, metadata: GrpcMetadata::default() })
        })
    }

    fn health_check(
        &self,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async { Ok(GrpcHealthCheck::healthy()) })
    }
}

// ── Cert helpers ─────────────────────────────────────────────────────────────

fn self_signed() -> (String, String) {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    (cert.cert.pem(), cert.key_pair.serialize_pem())
}

fn write_temp(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

// ── TLS client helpers ────────────────────────────────────────────────────────

#[derive(Debug)]
struct AcceptAnyServerCert;

impl rustls::client::danger::ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _: &[u8],
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

fn insecure_tls_connector() -> tokio_rustls::TlsConnector {
    // Force-install ring so the per-config builder below doesn't conflict
    // with other providers that might be registered during the test run.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCert))
    .with_no_client_auth();

    tokio_rustls::TlsConnector::from(Arc::new(cfg))
}

fn grpc_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

async fn grpc_call_tls(addr: SocketAddr, path: &str, payload: &[u8]) -> (u16, Option<String>, Bytes) {
    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let tls = insecure_tls_connector().connect(server_name, tcp).await.unwrap();
    let io  = TokioIo::new(tls);

    let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(io)
        .await
        .unwrap();
    tokio::spawn(conn);

    let req = Request::builder()
        .method("POST")
        .uri(format!("https://{addr}{path}"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(Full::new(grpc_frame(payload)))
        .unwrap();

    let resp      = sender.send_request(req).await.unwrap();
    let status    = resp.status().as_u16();
    let collected = resp.into_body().collect().await.unwrap();
    let grpc_status = collected
        .trailers()
        .and_then(|t| t.get("grpc-status"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    (status, grpc_status, collected.to_bytes())
}

// ── Server helpers ────────────────────────────────────────────────────────────

async fn start_tls_server(tls: IngressTlsConfig) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let server   = TonicGrpcServer::new("127.0.0.1:0", Arc::new(EchoHandler)).with_tls(tls);
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    (addr, tx)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// @covers: with_tls — server accepts a TLS connection, echoes the payload,
/// and returns grpc-status: 0
#[tokio::test]
async fn test_grpc_tls_server_echoes_payload_with_status_0() {
    let (cert_pem, key_pem) = self_signed();
    let cert_f = write_temp(&cert_pem);
    let key_f  = write_temp(&key_pem);

    let cfg = IngressTlsConfig::tls(
        cert_f.path().to_str().unwrap(),
        key_f.path().to_str().unwrap(),
    );
    let (addr, _shutdown) = start_tls_server(cfg).await;

    let payload = b"hello-tls";
    let (status, grpc_status, data) = grpc_call_tls(addr, "/pkg.Svc/Echo", payload).await;

    assert_eq!(status, 200, "expected HTTP 200");
    assert_eq!(grpc_status.as_deref(), Some("0"), "expected grpc-status 0");
    // Response body is a 5-byte gRPC frame header + echoed payload.
    assert!(data.len() >= 5 + payload.len());
    assert_eq!(&data[5..5 + payload.len()], payload, "payload not echoed");
}

/// @covers: with_tls — serve() fails fast when the cert file is missing
#[tokio::test]
async fn test_grpc_tls_server_returns_tls_error_for_missing_cert() {
    let cfg = IngressTlsConfig::tls("/no/such/cert.pem", "/no/such/key.pem");
    let server = TonicGrpcServer::new("127.0.0.1:0", Arc::new(EchoHandler)).with_tls(cfg);
    let err = server.serve(std::future::pending::<()>()).await;
    assert!(err.is_err(), "expected Tls error");
    let msg = err.unwrap_err().to_string();
    assert!(
        msg.contains("TLS") || msg.contains("cert"),
        "unexpected error: {msg}"
    );
}

/// @covers: with_tls — a plain gRPC server (no TLS) alongside a TLS server
/// continues to work over h2c
#[tokio::test]
async fn test_plain_grpc_server_unaffected_when_tls_server_runs_concurrently() {
    use bytes::Buf as _;

    // Plain h2c server
    let plain_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let plain_addr     = plain_listener.local_addr().unwrap();
    let (ptx, prx)     = oneshot::channel::<()>();
    let plain_server   = TonicGrpcServer::new("127.0.0.1:0", Arc::new(EchoHandler));
    tokio::spawn(async move {
        plain_server
            .serve_with_listener(plain_listener, async move { let _ = prx.await; })
            .await
            .unwrap();
    });

    // TLS server
    let (cert_pem, key_pem) = self_signed();
    let cert_f = write_temp(&cert_pem);
    let key_f  = write_temp(&key_pem);
    let cfg = IngressTlsConfig::tls(
        cert_f.path().to_str().unwrap(),
        key_f.path().to_str().unwrap(),
    );
    let (_tls_addr, _ttx) = start_tls_server(cfg).await;

    // The plain server still responds normally over h2c.
    let stream = tokio::net::TcpStream::connect(plain_addr).await.unwrap();
    let io     = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(io)
        .await
        .unwrap();
    tokio::spawn(conn);

    let req = Request::builder()
        .method("POST")
        .uri(format!("http://{plain_addr}/pkg.Svc/Echo"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(Full::new(grpc_frame(b"plain")))
        .unwrap();

    let resp = sender.send_request(req).await.unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let collected = resp.into_body().collect().await.unwrap();
    let grpc_status = collected
        .trailers()
        .and_then(|t| t.get("grpc-status"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    assert_eq!(grpc_status.as_deref(), Some("0"));

    let _ = ptx.send(());
}
