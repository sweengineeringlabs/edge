//! `swe-edge-ingress-http` — HTTP inbound domain (value objects + HttpInbound port).
#![allow(dead_code)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
