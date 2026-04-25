//! `swe_edge_egress` — outbound gateway adapters.
//!
//! Public surface is delegated entirely via `gateway/`. Consumers call
//! `swe_edge_egress::memory_database()`, `swe_edge_egress::Builder`, etc.
//! and receive `impl Trait` — never a named concrete type.

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
