//! `swe_edge_ingress` — inbound gateway adapters.
//!
//! Public surface is delegated entirely via `gateway/`. Consumers call
//! `swe_edge_ingress::file_input()`, `swe_edge_ingress::Builder`, etc.
//! and receive `impl Trait` — never a named concrete type.

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
