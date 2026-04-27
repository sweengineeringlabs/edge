//! # edge-proxy
//!
//! The L2 Proxy — dispatch facade sitting between ingress and domain.
//!
//! ## Concerns
//!
//! 1. **Job** — top-level entry called by the runtime.
//! 2. **Routing** — classify input into an intent.
//! 3. **Lifecycle** — health, background tasks, shutdown.
//!
//! ## Canonical usage shape
//!
//! ```ignore
//! use std::sync::Arc;
//! use edge_proxy::{Job, JobError, Router};
//! use edge_domain::{Handler, HandlerRegistry, new_handler_registry};
//!
//! // 1. Build a registry and register domain handlers.
//! let registry: Arc<HandlerRegistry<MyReq, MyResponse>> = new_handler_registry();
//! registry.register(Arc::new(MyHandler::new()));
//!
//! // 2. Implement Job backed by Router + HandlerRegistry.
//! // 3. Runtime holds Arc<dyn Job<MyReq, MyResponse>> and calls .run(req).
//! ```

mod api;
mod core;
mod gateway;

pub mod saf;

pub use saf::*;
