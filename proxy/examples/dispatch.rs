//! Proxy dispatch — Router classifies input, Job runs the handler, LifecycleMonitor reports health.
//!
//! Run:
//!     cargo run -p edge-proxy --example dispatch
//!
//! Demonstrates the full proxy dispatch path:
//!   request → Job::run → Router::route → HandlerRegistry::get → Handler::execute
//!
//! SEA layer boundaries kept explicit:
//!   - `edge_domain::` — Handler + HandlerRegistry contracts and their SAF factory
//!   - `edge_proxy::` — Job + Router + LifecycleMonitor contracts and their SAF factory

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use edge_domain::{Handler, HandlerError, HandlerRegistry, new_handler_registry};
use edge_proxy::{
    Job, JobError, Router, RoutingError,
    new_null_lifecycle_monitor,
};

// ── request / response types ──────────────────────────────────────────────────

#[derive(Debug)]
struct Request { command: String, payload: String }

#[derive(Debug)]
struct Response { handler: String, output:  String }

// ── Handler ───────────────────────────────────────────────────────────────────

struct EchoHandler;

#[async_trait]
impl Handler<Request, Response> for EchoHandler {
    fn id(&self)      -> &str { "echo" }
    fn pattern(&self) -> &str { "direct" }

    async fn execute(&self, req: Request) -> Result<Response, HandlerError> {
        Ok(Response { handler: self.id().into(), output: req.payload })
    }

    async fn health_check(&self) -> bool { true }
    fn as_any(&self) -> &dyn Any { self }
}

// ── Router ────────────────────────────────────────────────────────────────────

struct CommandRouter;

#[async_trait]
impl Router<String> for CommandRouter {
    async fn route(&self, input: &str) -> Result<String, RoutingError> {
        match input {
            "echo" | "ping" => Ok("echo".into()),
            _               => Err(RoutingError::NoMatch),
        }
    }
}

// ── Job ───────────────────────────────────────────────────────────────────────

struct DispatchJob {
    router:   Arc<dyn Router<String>>,
    registry: Arc<HandlerRegistry<Request, Response>>,
}

#[async_trait]
impl Job<Request, Response> for DispatchJob {
    async fn run(&self, req: Request) -> Result<Response, JobError> {
        let handler_id = self.router.route(&req.command).await?;
        let handler = self.registry
            .get(&handler_id)
            .ok_or_else(|| JobError::HandlerUnavailable(handler_id))?;
        Ok(handler.execute(req).await?)
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Domain: populate the handler registry via the SAF factory.
    let registry = new_handler_registry::<Request, Response>();
    registry.register(Arc::new(EchoHandler));

    // 2. Proxy: wire router + registry into a Job.
    let job: Arc<dyn Job<Request, Response>> = Arc::new(DispatchJob {
        router:   Arc::new(CommandRouter),
        registry: registry.clone(),
    });

    // 3. Dispatch — known command routes to the echo handler.
    let resp = job.run(Request { command: "echo".into(), payload: "hello proxy".into() }).await?;
    println!("echo  → handler={} output={}", resp.handler, resp.output);

    // 4. Dispatch — routing miss is surfaced as a JobError.
    let err = job.run(Request { command: "unknown".into(), payload: "".into() }).await.unwrap_err();
    println!("unknown → {err}");

    // 5. Lifecycle: null monitor reports Healthy out of the box.
    let lifecycle = new_null_lifecycle_monitor();
    let report    = lifecycle.health().await;
    println!("lifecycle overall → {:?}", report.overall);

    Ok(())
}
