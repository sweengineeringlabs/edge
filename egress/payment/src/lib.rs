//! `swe_edge_egress_payment` — payment outbound domain.

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
