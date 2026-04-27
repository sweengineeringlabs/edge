//! HTTPS echo server — AxumHttpServer with mutual TLS (mTLS).
//!
//! Generates a local CA, a server cert signed by that CA, and a client cert
//! signed by the same CA. The server requires every client to present a cert
//! issued by the CA; connections without a valid client cert are rejected
//! during the TLS handshake.
//!
//! Run:
//!     cargo run -p swe-edge-ingress --example https_echo_mtls
//!
//! Test with a valid client cert (paths are printed at startup):
//!     curl --cacert /tmp/ca.pem \
//!          --cert   /tmp/client.pem \
//!          --key    /tmp/client.key \
//!          https://127.0.0.1:8444/hello
//!
//! Test that a request without a client cert is rejected:
//!     curl -k https://127.0.0.1:8444/hello   # → TLS handshake failure

use std::io::Write as _;
use std::sync::Arc;

use rcgen::{BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair};
use swe_edge_ingress::{
    AxumHttpServer, HttpHealthCheck, HttpInbound, HttpInboundError, HttpInboundResult,
    HttpRequest, HttpResponse, IngressTlsConfig,
};

struct EchoHandler;

impl HttpInbound for EchoHandler {
    fn handle(
        &self,
        req: HttpRequest,
    ) -> futures::future::BoxFuture<'_, HttpInboundResult<HttpResponse>> {
        Box::pin(async move {
            let body = serde_json::json!({
                "method": req.method.to_string(),
                "url":    req.url,
            });
            let bytes = serde_json::to_vec_pretty(&body)
                .map_err(|e| HttpInboundError::Internal(e.to_string()))?;
            let mut resp = HttpResponse::new(200, bytes);
            resp.headers
                .insert("content-type".into(), "application/json; charset=utf-8".into());
            Ok(resp)
        })
    }

    fn health_check(
        &self,
    ) -> futures::future::BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

fn write_temp(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

struct MtlsBundle {
    ca_cert:     tempfile::NamedTempFile,
    server_cert: tempfile::NamedTempFile,
    server_key:  tempfile::NamedTempFile,
    client_cert: tempfile::NamedTempFile,
    client_key:  tempfile::NamedTempFile,
}

fn generate_mtls_bundle() -> MtlsBundle {
    // Root CA (self-signed, no SANs required).
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::new(vec![]).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();

    // Server cert — signed by CA, ServerAuth EKU, SAN = localhost.
    let server_key = KeyPair::generate().unwrap();
    let mut server_params =
        CertificateParams::new(vec!["localhost".to_string()]).unwrap();
    server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let server_cert = server_params
        .signed_by(&server_key, &ca_cert, &ca_key)
        .unwrap();

    // Client cert — signed by CA, ClientAuth EKU.
    let client_key = KeyPair::generate().unwrap();
    let mut client_params =
        CertificateParams::new(vec!["client".to_string()]).unwrap();
    client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    let client_cert = client_params
        .signed_by(&client_key, &ca_cert, &ca_key)
        .unwrap();

    MtlsBundle {
        ca_cert:     write_temp(&ca_cert.pem()),
        server_cert: write_temp(&server_cert.pem()),
        server_key:  write_temp(&server_key.serialize_pem()),
        client_cert: write_temp(&client_cert.pem()),
        client_key:  write_temp(&client_key.serialize_pem()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bundle = generate_mtls_bundle();

    let tls = IngressTlsConfig::mtls(
        bundle.server_cert.path().to_str().unwrap(),
        bundle.server_key.path().to_str().unwrap(),
        bundle.ca_cert.path().to_str().unwrap(),
    );

    println!("HTTPS (mTLS) echo server  →  https://127.0.0.1:8444");
    println!("  CA cert:     {}", bundle.ca_cert.path().display());
    println!("  Client cert: {}", bundle.client_cert.path().display());
    println!("  Client key:  {}", bundle.client_key.path().display());
    println!();
    println!("  # Authenticated request (succeeds):");
    println!(
        "  curl --cacert {} \\",
        bundle.ca_cert.path().display()
    );
    println!(
        "       --cert   {} \\",
        bundle.client_cert.path().display()
    );
    println!(
        "       --key    {} \\",
        bundle.client_key.path().display()
    );
    println!("       https://127.0.0.1:8444/hello");
    println!();
    println!("  # No client cert — rejected at TLS handshake:");
    println!("  curl -k https://127.0.0.1:8444/hello");
    println!();
    println!("Press Ctrl+C to stop.");

    let server =
        AxumHttpServer::new("127.0.0.1:8444", Arc::new(EchoHandler)).with_tls(tls);
    server
        .serve(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    println!("\nServer stopped.");
    Ok(())
}
