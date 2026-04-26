//! `swe_edge_daemon` — process-level runtime manager.

#![allow(dead_code)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
