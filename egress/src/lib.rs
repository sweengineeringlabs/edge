//! `swe_edge_egress` — outbound gateway adapters.
//!
//! Public surface is delegated entirely via `saf/`. Consumers call
//! `swe_edge_egress::database()`, `swe_edge_egress::http_client()`, etc.
//! and receive `impl Trait` — never a named concrete type.

mod api;
mod core;
mod gateway;
mod saf;

pub use saf::*;
