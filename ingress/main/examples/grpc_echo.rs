//! gRPC echo server — DaemonRunner → TonicGrpcServer → GrpcInbound.
//!
//! Run:
//!     cargo run -p swe-edge-ingress --example grpc_echo
//!
//! Test (requires grpcurl):
//!     grpcurl -plaintext -d '{}' 127.0.0.1:50051 echo.EchoService/Echo
//!
//! The server echoes every unary request's raw body back unchanged.

use std::sync::Arc;

use swe_edge_ingress::{
    DaemonRunner, GrpcHealthCheck, GrpcInbound, GrpcInboundResult, GrpcMetadata, GrpcRequest,
    GrpcResponse, TonicGrpcServer,
};

struct EchoHandler;

impl GrpcInbound for EchoHandler {
    fn handle_unary(
        &self,
        req: GrpcRequest,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            Ok(GrpcResponse {
                body:     req.body,
                metadata: GrpcMetadata::default(),
            })
        })
    }

    fn health_check(
        &self,
    ) -> futures::future::BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async { Ok(GrpcHealthCheck::healthy()) })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    DaemonRunner::new("grpc-echo")
        .with_bind("127.0.0.1:50051")
        .without_observability()
        .run(|ctx| async move {
            println!("gRPC echo server listening on {}", ctx.bind);
            println!("Raw bytes sent to any method are echoed back unchanged.");
            println!("Press Ctrl+C to stop.\n");

            let handler = Arc::new(EchoHandler);
            let server = TonicGrpcServer::new(ctx.bind.to_string(), handler);
            server
                .serve(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;

            println!("\nServer stopped.");
            Ok(())
        })
        .await
}
