//! # edge-controller
//!
//! The L2 Controller contract — a reusable 5-concern facade sitting between
//! an inbound gateway and a domain's execution units.
//!
//! ## The five concerns
//!
//! 1. **Job** — top-level entry called by the gateway.
//! 2. **Routing** — classify input into an intent.
//! 3. **Handlers** — uniform execution-unit contract.
//! 4. **Lifecycle** — health, background tasks, shutdown.
//! 5. **Gateway (boundary types)** — internal DTOs.
//!
//! ## Canonical usage shape
//!
//! ```ignore
//! use std::sync::Arc;
//! use edge_controller::{Handler, HandlerRegistry, new_handler_registry};
//!
//! // 1. Build a registry and register domain handlers.
//! let registry: Arc<HandlerRegistry<MyReq, MyResp>> = new_handler_registry();
//! registry.register(Arc::new(MyAgent::new()));
//!
//! // 2. Implement Job for your domain, backed by Router + Registry.
//! // 3. Gateway holds Arc<dyn Job<MyReq, MyResp>> and calls .run(req).
//! ```

mod api;
mod core;
mod gateway;

pub mod saf;

pub use saf::*;
