//! swe_http_cache — RFC-7234 HTTP cache middleware (wraps http-cache-reqwest with moka).
//!


#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
