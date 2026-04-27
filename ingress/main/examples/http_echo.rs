//! HTTP echo server — DaemonRunner → AxumHttpServer → HttpInbound.
//!
//! Run:
//!     cargo run -p swe-edge-ingress --example http_echo
//!
//! Test:
//!     curl http://127.0.0.1:8080/hello
//!     curl -X POST http://127.0.0.1:8080/data \
//!          -H 'Content-Type: application/json' \
//!          -d '{"msg": "hello world"}'

use std::sync::Arc;

use swe_edge_ingress::{
    AxumHttpServer, DaemonRunner, HttpHealthCheck, HttpInbound, HttpInboundError,
    HttpInboundResult, HttpRequest, HttpResponse,
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
            resp.headers.insert(
                "content-type".into(),
                "application/json; charset=utf-8".into(),
            );
            Ok(resp)
        })
    }

    fn health_check(
        &self,
    ) -> futures::future::BoxFuture<'_, HttpInboundResult<HttpHealthCheck>> {
        Box::pin(async { Ok(HttpHealthCheck::healthy()) })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    DaemonRunner::new("http-echo")
        .with_bind("127.0.0.1:8080")
        .without_observability()
        .run(|ctx| async move {
            println!("HTTP echo server listening on http://{}", ctx.bind);
            println!("  GET  http://{}/hello", ctx.bind);
            println!("  POST http://{}/data  (any JSON body)", ctx.bind);
            println!("Press Ctrl+C to stop.\n");

            let handler = Arc::new(EchoHandler);
            let server = AxumHttpServer::new(ctx.bind.to_string(), handler);
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
