//! swe_http_cache — RFC-7234 HTTP cache middleware (wraps http-cache-reqwest with moka).
//!
//! **Status: scaffolded, not yet implemented.** This crate's public
//! surface is stable in shape (SEA layers: api / core / saf) so
//! consumers can depend on it now and pick up behavior when impls
//! land. Tracked in the edge repo's next milestone.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;
mod core;
mod saf;

pub use saf::*;
