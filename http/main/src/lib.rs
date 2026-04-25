//! swe-http-main — workspace composition root.
//!
//! Assembles the configured HTTP middleware stack from the
//! individual middleware crates. Consumers use the `builder()`
//! entry point to obtain a pre-configured `reqwest_middleware`
//! client.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
