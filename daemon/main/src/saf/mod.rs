//! SAF layer — daemon public facade.

use std::sync::Arc;

use edge_controller::LifecycleMonitor;

use crate::core::DefaultRuntimeManager;

pub use crate::api::error::{RuntimeError, RuntimeResult};
pub use crate::api::traits::RuntimeManager;
pub use crate::api::types::{RuntimeConfig, RuntimeHealth, RuntimeStatus};
pub use crate::api::types::runtime_health::ComponentHealth;

/// Assemble a [`RuntimeManager`] from the supplied config and lifecycle monitor.
pub fn runtime_manager(
    config: RuntimeConfig,
    lifecycle: Arc<dyn LifecycleMonitor>,
) -> impl RuntimeManager {
    DefaultRuntimeManager::new(config, lifecycle)
}

/// Assemble a [`RuntimeManager`] with SWE defaults and the supplied lifecycle monitor.
pub fn default_runtime_manager(lifecycle: Arc<dyn LifecycleMonitor>) -> impl RuntimeManager {
    DefaultRuntimeManager::new(RuntimeConfig::default(), lifecycle)
}
