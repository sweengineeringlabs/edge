//! swe_http_cassette — VCR-style record/replay middleware for deterministic e2e tests.
//!


#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
pub(crate) mod gateway;
mod saf;

pub use saf::*;
