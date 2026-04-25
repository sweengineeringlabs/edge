//! `swe_edge_ingress` — inbound gateway adapters.
//!
//! Public surface is delegated entirely via `saf/`. Consumers call
//! `swe_edge_ingress::file_input()`, `swe_edge_ingress::http_input()`, etc.
//! and receive `impl Trait` — never a named concrete type.

mod api;
mod core;
mod gateway;
mod saf;

pub use saf::*;
