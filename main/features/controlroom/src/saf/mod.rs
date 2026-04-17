//! SAF (Static Access Facade) layer — stable function entrypoints.
//!
//! Re-exports the concern traits and contract-level types from `api/`.
//! Factory functions provide stable entry points for constructing the
//! default building blocks.

mod factory;

// Factory functions
pub use factory::{new_handler_registry, new_null_lifecycle_monitor};

// Concern traits
pub use crate::api::job::Job;
pub use crate::api::router::Router;
pub use crate::api::handler::Handler;
pub use crate::api::lifecycle_monitor::LifecycleMonitor;

// Error types
pub use crate::api::error::{HandlerError, JobError, LifecycleError, RoutingError};

// Health types
pub use crate::api::health::{ComponentHealth, HealthReport, HealthStatus};

// Contract-level building blocks (shared default types)
pub use crate::api::handler_registry::HandlerRegistry;
