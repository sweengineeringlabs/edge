//! ConfigLoader trait — layered config loading contract.

use crate::api::config::ConfigError;
use crate::api::types::RuntimeConfig;

/// Loads `RuntimeConfig` from the layered config chain:
/// `default.toml` → `application.toml` → `tenants/<id>.toml` → env vars.
pub trait ConfigLoader: Send + Sync {
    /// Load config for a single-tenant (or unscoped) deployment.
    fn load(&self) -> Result<RuntimeConfig, ConfigError>;

    /// Load config scoped to a specific tenant, layering
    /// `tenants/<tenant_id>.toml` on top of `application.toml`.
    fn load_for_tenant(&self, tenant_id: &str) -> Result<RuntimeConfig, ConfigError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loader_is_object_safe() {
        fn _assert(_: &dyn ConfigLoader) {}
    }
}
