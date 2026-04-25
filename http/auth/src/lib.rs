//! swe_edge_http_auth — HTTP auth middleware for reqwest-middleware.
//!
//! Attaches bearer tokens, basic-auth credentials, or custom
//! API-key headers to outbound HTTP requests. Credentials are
//! resolved from environment variables at config-load time; the
//! config itself stores only the env-var NAME, never the raw
//! credential.
//!


#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod gateway;
mod saf;

pub use gateway::*;
