//! Gateway layer — inbound/outbound adapters.
//!
//! Re-exports the SAF public surface so `lib.rs` can do
//! `pub use gateway::*` as the single delegation point
//! (SEA rule 54 — gateway is the public entry boundary).

pub use crate::saf::*;

pub(crate) mod input;
pub(crate) mod output;
