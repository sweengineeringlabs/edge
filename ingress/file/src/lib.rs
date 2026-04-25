//! `swe-edge-ingress-file` — file inbound domain (value objects + FileInbound port + LocalFileSource).
#![allow(dead_code)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
