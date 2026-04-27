//! Handler registry — register, resolve, execute, and deregister domain Handlers.
//!
//! Run:
//!     cargo run -p edge-domain --example handler_registry
//!
//! Demonstrates the full domain execution-unit contract:
//!   new_handler_registry → register → get → execute → health_check → deregister
//!
//! SEA constraint: all imports come from the `edge_domain` SAF surface.
//! Concrete handler structs are defined locally — consumers never name the
//! crate-internal implementations.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use edge_domain::{Handler, HandlerError, new_handler_registry};

// ── concrete Handler ──────────────────────────────────────────────────────────

struct GreetHandler;

#[async_trait]
impl Handler<String, String> for GreetHandler {
    fn id(&self)      -> &str { "greet" }
    fn pattern(&self) -> &str { "direct" }

    async fn execute(&self, req: String) -> Result<String, HandlerError> {
        if req.is_empty() {
            return Err(HandlerError::InvalidRequest("name must not be empty".into()));
        }
        Ok(format!("Hello, {req}!"))
    }

    async fn health_check(&self) -> bool { true }
    fn as_any(&self) -> &dyn Any { self }
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Build an empty registry via the SAF factory.
    let registry = new_handler_registry::<String, String>();
    assert!(registry.is_empty());

    // 2. Register the handler.
    registry.register(Arc::new(GreetHandler));
    println!("registered:   {:?}", registry.list_ids());
    assert_eq!(registry.len(), 1);

    // 3. Resolve and execute — happy path.
    let handler = registry.get("greet").expect("handler must be present");
    let resp    = handler.execute("world".into()).await?;
    println!("execute       → {resp}");

    // 4. Execute — handler rejects invalid input.
    let err = handler.execute("".into()).await.unwrap_err();
    println!("empty name    → {err}");

    // 5. Health probe.
    let healthy = handler.health_check().await;
    println!("health_check  → {healthy}");
    assert!(healthy);

    // 6. Deregister — registry returns None on subsequent lookups.
    let removed = registry.deregister("greet");
    assert!(removed);
    assert!(registry.get("greet").is_none());
    println!("after remove: {:?}", registry.list_ids());

    Ok(())
}
