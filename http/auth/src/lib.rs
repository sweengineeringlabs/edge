//! swe_http_auth — HTTP auth middleware for reqwest-middleware.
//!
//! Attaches bearer tokens, basic-auth credentials, or custom
//! API-key headers to outbound HTTP requests. Credentials are
//! resolved from environment variables at config-load time; the
//! config itself stores only the env-var NAME, never the raw
//! credential.
//!
//! **Status: scaffolded, not yet implemented.** Schema + config
//! surface is stable; the `Middleware` impl that actually
//! attaches the header lands in follow-up work.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod saf;

pub use saf::*;
