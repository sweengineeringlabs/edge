//! SAF layer — daemon public facade.

use std::sync::Arc;

use edge_controller::LifecycleMonitor;

use crate::core::DefaultRuntimeManager;

pub use crate::api::error::{RuntimeError, RuntimeResult};
pub use crate::api::runtime_manager::RuntimeManager;
pub use crate::api::types::{RuntimeConfig, RuntimeHealth, RuntimeStatus};
pub use crate::api::types::runtime_health::ComponentHealth;
pub use crate::gateway::input::IngressGateway;
pub use crate::gateway::output::EgressGateway;

/// Assemble a [`RuntimeManager`] from the supplied config, ingress, egress,
/// and lifecycle monitor.
pub fn runtime_manager(
    config:    RuntimeConfig,
    ingress:   IngressGateway,
    egress:    EgressGateway,
    lifecycle: Arc<dyn LifecycleMonitor>,
) -> impl RuntimeManager {
    DefaultRuntimeManager::new(config, ingress, egress, lifecycle)
}
