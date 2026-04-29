//! Gateway layer — public entry boundary.
//!
//! Re-exports the SAF facade so `lib.rs` does
//! `pub use gateway::*` (SEA rule 54).

pub use crate::saf::*;
