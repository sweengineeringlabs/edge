//! HTTPS echo server — AxumHttpServer with server-side TLS.
//!
//! Generates a self-signed certificate at startup (requires no pre-existing
//! files) and serves HTTPS on 127.0.0.1:8443. The cert path is printed so you
//! can test with or without `-k`.
//!
//! Run:
//!     cargo run -p swe-edge-ingress --example https_echo
//!
//! Test:
//!     curl -k https://127.0.0.1:8443/hello
//!     curl -k -X POST https://127.0.0.1:8443/data \
//!          -H 'Content-Type: application/json' \
//!          -d '{"msg":"hello"}'
//!
//! To verify the cert instead of skipping it:
//!     curl --cacert <printed path> https://127.0.0.1:8443/hello

use std::io::Write as _;
use std::sync::Arc;

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
                "method":  req.method.to_string(),
                "url":     req.url,
                "headers": req.headers,
                "query":   req.query,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate a fresh self-signed cert for localhost.
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let cert_f = write_temp(&cert.cert.pem());
    let key_f  = write_temp(&cert.key_pair.serialize_pem());

    let tls = IngressTlsConfig::tls(
        cert_f.path().to_str().unwrap(),
        key_f.path().to_str().unwrap(),
    );

    println!("HTTPS echo server  →  https://127.0.0.1:8443");
    println!("  self-signed cert: {}", cert_f.path().display());
    println!();
    println!("  curl -k https://127.0.0.1:8443/hello");
    println!(
        "  curl --cacert {} https://127.0.0.1:8443/hello",
        cert_f.path().display()
    );
    println!();
    println!("Press Ctrl+C to stop.");

    let server = AxumHttpServer::new("127.0.0.1:8443", Arc::new(EchoHandler)).with_tls(tls);
    server
        .serve(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    println!("\nServer stopped.");
    Ok(())
}
