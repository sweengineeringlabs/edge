//! `swe_edge_egress_grpc` — gRPC outbound domain.

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
