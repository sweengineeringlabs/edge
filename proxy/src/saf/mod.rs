//! SAF layer — proxy public facade.

mod factory;

// Factory functions
pub use factory::new_null_lifecycle_monitor;

// Concern traits
pub use crate::api::job::Job;
pub use crate::api::router::Router;
pub use crate::api::lifecycle_monitor::LifecycleMonitor;

// Error types
pub use crate::api::error::{HandlerError, JobError, LifecycleError, RoutingError};

// Health types
pub use crate::api::health::{ComponentHealth, HealthReport, HealthStatus};
