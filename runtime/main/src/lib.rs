//! `swe_edge_runtime` — process-level runtime manager.

#![allow(dead_code)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
