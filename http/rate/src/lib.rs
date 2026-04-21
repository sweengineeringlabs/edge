//! swe_http_rate — Client-side rate-limiter middleware — token bucket per host.
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
