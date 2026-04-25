//! Barrel re-export of the 5-Concern ControlRoom traits.
//!
//! Consumers who want a single import path can write
//! `use swe_edge_controlroom::api::traits::*;` and get all concern traits.

pub use super::job::Job;
pub use super::router::Router;
pub use super::handler::Handler;
pub use super::lifecycle_monitor::LifecycleMonitor;

/// Marker type naming the five concerns for discoverability in docs.
///
/// Traits implementing each concern:
/// 1. **Job** — [`Job`]
/// 2. **Routing** — [`Router`]
/// 3. **Handlers** — [`Handler`]
/// 4. **Lifecycle** — [`LifecycleMonitor`]
/// 5. **Gateway (boundary types)** — `crate::gateway` module (internal)
pub struct ControlRoomPattern;
