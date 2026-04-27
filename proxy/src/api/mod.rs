//! API layer — proxy dispatch concerns.
//!
//! | File | Concern |
//! |------|---------|
//! | [`job`] | Job — top-level entry called by the runtime |
//! | [`router`] | Routing — classify input into an intent |
//! | [`lifecycle_monitor`] | Lifecycle — health, background tasks, shutdown |

pub mod error;
pub mod health;

pub mod job;
pub mod router;
pub mod lifecycle_monitor;

pub mod traits;
