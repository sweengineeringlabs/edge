//! swe_http_breaker — Circuit-breaker middleware — fail fast on degraded upstreams.
//!


#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
pub(crate) mod gateway;
mod saf;

pub use saf::*;
