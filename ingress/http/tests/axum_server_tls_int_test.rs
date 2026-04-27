//! Integration tests for AxumHttpServer TLS and mTLS.
//!
//! Each test generates a fresh self-signed cert with `rcgen`, starts an HTTPS
//! server on an ephemeral port, and connects using a raw tokio-rustls client
//! that accepts any server certificate.

use std::io::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use swe_edge_ingress_http::{
    AxumHttpServer, HttpHealthCheck, HttpInbound, HttpInboundResult, HttpRequest, HttpResponse,
    IngressTlsConfig,
};

// ── Stub handler ─────────────────────────────────────────────────────────────

struct EchoHandler;

impl HttpInbound for EchoHandler {
    fn handle(&self, req: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async move {
            let body = format!("{} {}", req.method, req.url).into_bytes();
            Ok(HttpResponse::new(200, body))
        })
    }

    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

// ── Cert helpers ─────────────────────────────────────────────────────────────

/// Returns `(cert_pem, key_pem)` for a fresh self-signed certificate valid
/// for `localhost`.
fn self_signed() -> (String, String) {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    (cert.cert.pem(), cert.key_pair.serialize_pem())
}

fn write_temp(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

// ── TLS test client ───────────────────────────────────────────────────────────

/// A minimal rustls `ServerCertVerifier` that accepts any certificate.
/// Only used in tests — never in production code.
#[derive(Debug)]
struct AcceptAnyServerCert;

impl rustls::client::danger::ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
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

/// Builds a `tokio_rustls::TlsConnector` that skips certificate verification.
fn insecure_connector() -> tokio_rustls::TlsConnector {
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

/// Send a minimal HTTP/1.1 GET over a TLS stream; return the status line.
async fn https_get(addr: SocketAddr, path: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let mut tls = insecure_connector().connect(server_name, tcp).await.unwrap();

    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    tls.write_all(req.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    tls.read_to_end(&mut buf).await.unwrap();

    // Return just the status line (first line of the response).
    String::from_utf8_lossy(&buf)
        .lines()
        .next()
        .unwrap_or("")
        .to_string()
}

// ── Server helpers ────────────────────────────────────────────────────────────

async fn start_tls_server(tls: IngressTlsConfig) -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel::<()>();

    let server = AxumHttpServer::new(addr.to_string(), Arc::new(EchoHandler)).with_tls(tls);
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });

    // Brief pause so the accept loop is live before the test sends a request.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    (addr, tx)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// @covers: with_tls — server accepts a TLS connection and returns 200
#[tokio::test]
async fn test_https_server_returns_200_for_get_request() {
    let (cert_pem, key_pem) = self_signed();
    let cert_f = write_temp(&cert_pem);
    let key_f  = write_temp(&key_pem);

    let cfg = IngressTlsConfig::tls(
        cert_f.path().to_str().unwrap(),
        key_f.path().to_str().unwrap(),
    );
    let (addr, _shutdown) = start_tls_server(cfg).await;

    let status_line = https_get(addr, "/hello").await;
    assert!(
        status_line.contains("200"),
        "expected HTTP 200, got: {status_line}"
    );
}

/// @covers: with_tls — handler response body is returned over TLS
#[tokio::test]
async fn test_https_server_echoes_method_and_path_in_response_body() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (cert_pem, key_pem) = self_signed();
    let cert_f = write_temp(&cert_pem);
    let key_f  = write_temp(&key_pem);

    let cfg = IngressTlsConfig::tls(
        cert_f.path().to_str().unwrap(),
        key_f.path().to_str().unwrap(),
    );
    let (addr, _shutdown) = start_tls_server(cfg).await;

    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let mut tls = insecure_connector().connect(server_name, tcp).await.unwrap();

    tls.write_all(b"GET /echo-me HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();

    let mut buf = Vec::new();
    tls.read_to_end(&mut buf).await.unwrap();

    let body = String::from_utf8_lossy(&buf);
    assert!(
        body.contains("GET") && body.contains("/echo-me"),
        "response did not echo method/path: {body}"
    );
}

/// @covers: with_tls — serve() builds the acceptor eagerly; missing cert returns Tls error
#[tokio::test]
async fn test_https_server_returns_tls_error_for_missing_cert_file() {
    let cfg = IngressTlsConfig::tls("/no/such/cert.pem", "/no/such/key.pem");
    let server = AxumHttpServer::new("127.0.0.1:0", Arc::new(EchoHandler)).with_tls(cfg);
    let err = server.serve(std::future::pending::<()>()).await;
    assert!(
        err.is_err(),
        "expected TLS error for missing cert, got Ok"
    );
    let msg = err.unwrap_err().to_string();
    assert!(
        msg.contains("TLS") || msg.contains("cert"),
        "unexpected error message: {msg}"
    );
}

/// @covers: with_tls — plain (non-TLS) server still works after adding TLS to a different instance
#[tokio::test]
async fn test_plain_server_unaffected_when_tls_server_runs_concurrently() {
    // Plain server
    let plain_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let plain_addr     = plain_listener.local_addr().unwrap();
    let (plain_tx, plain_rx) = oneshot::channel::<()>();
    let plain_server = AxumHttpServer::new(plain_addr.to_string(), Arc::new(EchoHandler));
    tokio::spawn(async move {
        plain_server
            .serve_with_listener(plain_listener, async move { let _ = plain_rx.await; })
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
    let (_tls_addr, _tls_tx) = start_tls_server(cfg).await;

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // The plain server should still respond over HTTP.
    let resp = reqwest::get(format!("http://{plain_addr}/check")).await.unwrap();
    assert_eq!(resp.status(), 200);

    let _ = plain_tx.send(());
}
