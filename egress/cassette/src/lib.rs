//! swe_edge_egress_cassette — VCR-style record/replay middleware for deterministic e2e tests.
//!


#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
