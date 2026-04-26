//! `swe-edge-ingress-grpc` — gRPC inbound domain (value objects + GrpcInbound port).
#![allow(dead_code)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
