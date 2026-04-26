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
///
/// The config directory is resolved from `SWE_EDGE_CONFIG_DIR` or
/// defaults to `config/` relative to the working directory.
/// Consumer apps should prefer [`load_config_from`] to supply their
/// own path explicitly.
pub fn load_config() -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::new().load()
}

/// Load config from an explicit directory.
///
/// Identical layer chain to [`load_config`] but reads
/// `<dir>/application.toml` instead of relying on env or cwd.
pub fn load_config_from(dir: impl Into<std::path::PathBuf>) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::with_dir(dir).load()
}

/// Load config scoped to a tenant
/// (`default.toml` → `application.toml` → `tenants/<id>.toml` → env vars).
///
/// See [`load_tenant_config_from`] for the consumer-app variant.
pub fn load_tenant_config(tenant_id: &str) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::new().load_for_tenant(tenant_id)
}

/// Load tenant config from an explicit directory.
///
/// Reads `<dir>/application.toml` and `<dir>/tenants/<tenant_id>.toml`.
/// Intended for consumer apps that own their config directory layout.
pub fn load_tenant_config_from(
    tenant_id: &str,
    dir: impl Into<std::path::PathBuf>,
) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::with_dir(dir).load_for_tenant(tenant_id)
}

/// Load config following the XDG Base Directory specification.
///
/// Layer chain (last wins):
/// - `$XDG_CONFIG_DIRS/<app_name>/application.toml` (system-wide, default `/etc/xdg/`)
/// - `$XDG_CONFIG_HOME/<app_name>/application.toml` (user-level, default `~/.config/`)
/// - `$SWE_EDGE_CONFIG_DIR/application.toml` (explicit override, if set)
/// - `SWE_EDGE_*` environment variables (always top priority)
pub fn load_config_xdg(app_name: &str) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::xdg(app_name).load()
}

/// Load tenant config following the XDG Base Directory specification.
///
/// Same XDG layer chain as [`load_config_xdg`], with
/// `tenants/<tenant_id>.toml` applied on top at the highest-priority
/// directory where it exists.
pub fn load_tenant_config_xdg(
    app_name: &str,
    tenant_id: &str,
) -> Result<RuntimeConfig, ConfigError> {
    DefaultConfigLoader::xdg(app_name).load_for_tenant(tenant_id)
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
