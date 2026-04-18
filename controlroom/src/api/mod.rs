//! Public API layer — the 5-Concern ControlRoom pattern.
//!
//! Each concern lives in its own file (SEA rule 108):
//!
//! | File | Concern |
//! |------|---------|
//! | [`job`] | Job — top-level entry called by the gateway |
//! | [`router`] | Routing — classify input into an intent |
//! | [`handler`] | Handlers — uniform execution-unit contract |
//! | [`lifecycle_monitor`] | Lifecycle — health, background tasks, shutdown |
//! | `crate::gateway` | Gateway (boundary types, internal) |
//!
//! See `docs/conventions/crate-naming.md` for the architectural convention.

pub mod error;
pub mod health;

pub mod job;
pub mod router;
pub mod handler;
pub mod handler_registry;
pub mod lifecycle_monitor;

pub mod traits;
