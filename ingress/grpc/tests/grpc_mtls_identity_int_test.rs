//! Issue #5 acceptance gate (4/4): mTLS peer identity flows through to
//! `GrpcMetadata` under documented reserved keys.
//!
//! Generates a CA + server cert + client cert with `rcgen`, runs an
//! mTLS-required server, and connects with a client that presents
//! the client cert.  The handler captures the request metadata and
//! we assert the documented `x-edge-peer-*` keys are populated with
//! data that came from the client's leaf certificate.

use std::io::Write as _;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use http::Request;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use swe_edge_ingress_grpc::{
    GrpcHealthCheck, GrpcInbound, GrpcInboundResult, GrpcMetadata, GrpcRequest, GrpcResponse,
    IngressTlsConfig, TonicGrpcServer, PEER_CERT_FINGERPRINT_SHA256, PEER_CN, PEER_SAN_DNS,
};

// ── Capturing handler ───────────────────────────────────────────────────────

struct CapturingHandler {
    captured: Arc<Mutex<Option<GrpcMetadata>>>,
}

impl GrpcInbound for CapturingHandler {
    fn handle_unary(
        &self,
        req: GrpcRequest,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        let captured = self.captured.clone();
        Box::pin(async move {
            *captured.lock().unwrap() = Some(req.metadata.clone());
            Ok(GrpcResponse { body: req.body, metadata: GrpcMetadata::default() })
        })
    }

    fn health_check(
        &self,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async { Ok(GrpcHealthCheck::healthy()) })
    }
}

// ── Cert helpers ────────────────────────────────────────────────────────────

fn write_temp(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

struct MtlsMaterial {
    server_cert_pem: String,
    server_key_pem:  String,
    client_ca_pem:   String,
    client_cert_pem: String,
    client_key_pem:  String,
}

fn build_mtls_material() -> MtlsMaterial {
    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, IsCa, BasicConstraints};

    // Root CA used to sign both the server and client leaves.
    let mut ca_params = CertificateParams::new(Vec::new()).unwrap();
    ca_params.is_ca   = IsCa::Ca(BasicConstraints::Unconstrained);
    let mut ca_dn = DistinguishedName::new();
    ca_dn.push(DnType::CommonName, "swe-edge-test-root");
    ca_params.distinguished_name = ca_dn;
    let ca_key  = KeyPair::generate().unwrap();
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();

    // Server leaf (CN=localhost, DNS=localhost).
    let mut server_params = CertificateParams::new(vec!["localhost".to_string()]).unwrap();
    let mut server_dn = DistinguishedName::new();
    server_dn.push(DnType::CommonName, "localhost");
    server_params.distinguished_name = server_dn;
    let server_key  = KeyPair::generate().unwrap();
    let server_cert = server_params.signed_by(&server_key, &ca_cert, &ca_key).unwrap();

    // Client leaf — CN + DNS SAN we will assert on later.
    let mut client_params = CertificateParams::new(vec!["client.svc.local".to_string()]).unwrap();
    let mut client_dn = DistinguishedName::new();
    client_dn.push(DnType::CommonName, "edge-test-client");
    client_params.distinguished_name = client_dn;
    let client_key  = KeyPair::generate().unwrap();
    let client_cert = client_params.signed_by(&client_key, &ca_cert, &ca_key).unwrap();

    MtlsMaterial {
        server_cert_pem: server_cert.pem(),
        server_key_pem:  server_key.serialize_pem(),
        client_ca_pem:   ca_cert.pem(),
        client_cert_pem: client_cert.pem(),
        client_key_pem:  client_key.serialize_pem(),
    }
}

// ── TLS client (presents client cert) ───────────────────────────────────────

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

fn mtls_client_connector(client_cert_pem: &str, client_key_pem: &str) -> tokio_rustls::TlsConnector {
    let _ = rustls::crypto::ring::default_provider().install_default();

    use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};

    let cert_chain: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut client_cert_pem.as_bytes())
            .filter_map(Result::ok)
            .collect();
    let key = PrivateKeyDer::from_pem_slice(client_key_pem.as_bytes())
        .expect("client key parses");

    let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCert))
    .with_client_auth_cert(cert_chain, key)
    .expect("client auth cert");

    let mut cfg = cfg;
    cfg.alpn_protocols = vec![b"h2".to_vec()];

    tokio_rustls::TlsConnector::from(Arc::new(cfg))
}

fn grpc_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

async fn mtls_call(
    addr:        SocketAddr,
    path:        &str,
    payload:     &[u8],
    client_cert: &str,
    client_key:  &str,
) {
    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let connector   = mtls_client_connector(client_cert, client_key);
    let tls = connector.connect(server_name, tcp).await.expect("TLS handshake");
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
    let resp      = sender.send_request(req).await.expect("response");
    let _collected = resp.into_body().collect().await.unwrap();
}

/// @covers: end-to-end mTLS — peer-identity keys appear in handler
/// metadata, derived from the client's leaf certificate.
/// Issue #5 acceptance gate (4/4).
#[tokio::test]
async fn mtls_peer_identity_flows_through_to_handler_metadata_int_test() {
    let m = build_mtls_material();
    let server_cert_f = write_temp(&m.server_cert_pem);
    let server_key_f  = write_temp(&m.server_key_pem);
    let client_ca_f   = write_temp(&m.client_ca_pem);

    let tls_cfg = IngressTlsConfig::mtls(
        server_cert_f.path().to_str().unwrap(),
        server_key_f.path().to_str().unwrap(),
        client_ca_f.path().to_str().unwrap(),
    );
    let captured: Arc<Mutex<Option<GrpcMetadata>>> = Arc::new(Mutex::new(None));
    let handler = Arc::new(CapturingHandler { captured: captured.clone() });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let server   = TonicGrpcServer::new("127.0.0.1:0", handler)
        .with_tls(tls_cfg)
        .allow_unauthenticated(true);
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    mtls_call(addr, "/pkg.Svc/Echo", b"identity-probe", &m.client_cert_pem, &m.client_key_pem).await;

    let snapshot = captured.lock().unwrap().clone().expect("handler should have captured metadata");
    let cn   = snapshot.headers.get(PEER_CN).cloned();
    let dns  = snapshot.headers.get(PEER_SAN_DNS).cloned();
    let fp   = snapshot.headers.get(PEER_CERT_FINGERPRINT_SHA256).cloned();

    assert_eq!(
        cn.as_deref(),
        Some("edge-test-client"),
        "peer CN should reach the handler under the documented key",
    );
    assert!(
        dns.as_deref().map(|s| s.contains("client.svc.local")).unwrap_or(false),
        "peer DNS SAN should reach the handler (got {dns:?})",
    );
    let fp = fp.expect("fingerprint must always be set under documented key");
    assert_eq!(fp.len(), 64, "fingerprint must be 64 hex chars (SHA-256)");
    assert!(fp.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "fingerprint must be lower-hex (got {fp})");

    let _ = tx.send(());
}

/// @covers: spoof-resistance — a client sending its own
/// `x-edge-peer-*` headers over mTLS sees them STRIPPED by the server
/// before the handler runs.
#[tokio::test]
async fn mtls_server_strips_spoofed_peer_headers_int_test() {
    let m = build_mtls_material();
    let server_cert_f = write_temp(&m.server_cert_pem);
    let server_key_f  = write_temp(&m.server_key_pem);
    let client_ca_f   = write_temp(&m.client_ca_pem);

    let tls_cfg = IngressTlsConfig::mtls(
        server_cert_f.path().to_str().unwrap(),
        server_key_f.path().to_str().unwrap(),
        client_ca_f.path().to_str().unwrap(),
    );
    let captured: Arc<Mutex<Option<GrpcMetadata>>> = Arc::new(Mutex::new(None));
    let handler = Arc::new(CapturingHandler { captured: captured.clone() });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr     = listener.local_addr().unwrap();
    let server   = TonicGrpcServer::new("127.0.0.1:0", handler)
        .with_tls(tls_cfg)
        .allow_unauthenticated(true);
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        server
            .serve_with_listener(listener, async move { let _ = rx.await; })
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let connector   = mtls_client_connector(&m.client_cert_pem, &m.client_key_pem);
    let tls = connector.connect(server_name, tcp).await.expect("TLS handshake");
    let io  = TokioIo::new(tls);
    let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
        .handshake(io)
        .await
        .unwrap();
    tokio::spawn(conn);

    // Spoof the peer-identity headers on the wire — the server must
    // ignore them and use its own derived values.
    let req = Request::builder()
        .method("POST")
        .uri(format!("https://{addr}/pkg.Svc/Echo"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .header("x-edge-peer-cn", "spoofed-admin")
        .header("x-edge-peer-identity", "CN=spoofed-admin")
        .body(Full::new(grpc_frame(b"spoof-attempt")))
        .unwrap();
    let resp = sender.send_request(req).await.unwrap();
    let _    = resp.into_body().collect().await.unwrap();

    let snapshot = captured.lock().unwrap().clone().expect("metadata captured");
    let cn   = snapshot.headers.get(PEER_CN).cloned();

    assert_eq!(
        cn.as_deref(),
        Some("edge-test-client"),
        "spoofed `x-edge-peer-cn` must be replaced by the server-derived value",
    );

    let _ = tx.send(());
}
