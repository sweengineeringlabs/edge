//! swe_edge_egress_rate — Client-side rate-limiter middleware — token bucket per host.
//!


#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
