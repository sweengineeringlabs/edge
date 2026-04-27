//! gRPC-over-TLS echo server — TonicGrpcServer with IngressTlsConfig::tls().
//!
//! Generates a self-signed certificate at startup and serves gRPC over TLS on
//! 127.0.0.1:50443. Every unary request's raw body is echoed back unchanged.
//!
//! Run:
//!     cargo run -p swe-edge-ingress --example grpcs_echo
//!
//! Test (grpcurl):
//!     grpcurl -insecure -d '' 127.0.0.1:50443 echo.EchoService/Echo
//!
//! To verify the cert instead of skipping it:
//!     grpcurl -cacert <printed path> -d '' 127.0.0.1:50443 echo.EchoService/Echo

use std::io::Write as _;
use std::sync::Arc;

use swe_edge_ingress::{
    GrpcHealthCheck, GrpcInbound, GrpcInboundResult, GrpcMetadata, GrpcRequest, GrpcResponse,
    IngressTlsConfig, TonicGrpcServer,
};

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

    println!("gRPC-over-TLS echo server  →  127.0.0.1:50443");
    println!("  self-signed cert: {}", cert_f.path().display());
    println!();
    println!("  grpcurl -insecure -d '' 127.0.0.1:50443 echo.EchoService/Echo");
    println!(
        "  grpcurl -cacert {} -d '' 127.0.0.1:50443 echo.EchoService/Echo",
        cert_f.path().display()
    );
    println!();
    println!("Press Ctrl+C to stop.");

    let server =
        TonicGrpcServer::new("127.0.0.1:50443", Arc::new(EchoHandler)).with_tls(tls);
    server
        .serve(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    println!("\nServer stopped.");
    Ok(())
}
