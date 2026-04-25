//! swe_http_retry — Opinionated retry middleware (wraps reqwest-retry with SWE defaults).
//!


#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
pub(crate) mod gateway;
mod saf;

pub use saf::*;
