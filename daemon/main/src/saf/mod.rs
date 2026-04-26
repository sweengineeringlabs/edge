//! SAF layer — daemon public facade.

use std::sync::Arc;

use edge_controller::LifecycleMonitor;

use crate::core::{DefaultConfigLoader, DefaultRuntimeManager};

pub use crate::api::config::ConfigError;
pub use crate::api::config_loader::ConfigLoader;
pub use crate::api::error::{RuntimeError, RuntimeResult};
pub use crate::api::runtime_manager::RuntimeManager;
pub use crate::api::types::{RuntimeConfig, RuntimeHealth, RuntimeStatus};
pub use crate::api::types::runtime_health::ComponentHealth;
pub use crate::gateway::input::IngressGateway;
pub use crate::gateway::output::EgressGateway;

/// Load config using the default layered chain
/// (`default.toml` → `application.toml` → env vars).
pub fn load_config() -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::new().load()
}

/// Load config scoped to a tenant
/// (`default.toml` → `application.toml` → `tenants/<id>.toml` → env vars).
pub fn load_tenant_config(tenant_id: &str) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::new().load_for_tenant(tenant_id)
}

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
