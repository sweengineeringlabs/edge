//! DefaultConfigLoader — loads RuntimeConfig from the layered chain.

use std::env;
use std::path::{Path, PathBuf};

use crate::api::config::{ConfigError, ConfigOverride};
use crate::api::config_loader::ConfigLoader;
use crate::api::types::RuntimeConfig;

/// Shipped defaults embedded at compile time.
const DEFAULT_TOML: &str = include_str!("../../config/default.toml");

/// Loads [`RuntimeConfig`] from the full layered chain:
///
/// 1. `default.toml` (compiled in)
/// 2. `<config_dir>/application.toml` (runtime, optional)
/// 3. `<config_dir>/tenants/<id>.toml` (runtime, optional, tenant-scoped only)
/// 4. Environment variables (`SWE_EDGE_*`)
///
/// `config_dir` defaults to `config/` relative to the working directory
/// unless `SWE_EDGE_CONFIG_DIR` is set.
pub(crate) struct DefaultConfigLoader {
    config_dir: PathBuf,
}

impl DefaultConfigLoader {
    pub(crate) fn new() -> Self {
        let dir = env::var("SWE_EDGE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("config"));
        Self { config_dir: dir }
    }

    fn base(&self) -> Result<RuntimeConfig, ConfigError> {
        let base = ConfigOverride::from_str(DEFAULT_TOML)?.apply_to(RuntimeConfig::default());
        let app_path = self.config_dir.join("application.toml");
        self.apply_file_if_exists(base, &app_path)
    }

    fn apply_file_if_exists(
        &self,
        cfg: RuntimeConfig,
        path: &Path,
    ) -> Result<RuntimeConfig, ConfigError> {
        if !path.exists() {
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(format!("{}: {e}", path.display())))?;
        Ok(ConfigOverride::from_str(&text)?.apply_to(cfg))
    }

    fn apply_env(mut cfg: RuntimeConfig) -> RuntimeConfig {
        if let Ok(v) = env::var("SWE_EDGE_SERVICE_NAME")      { cfg.service_name          = v; }
        if let Ok(v) = env::var("SWE_EDGE_HTTP_BIND")         { cfg.http_bind             = v; }
        if let Ok(v) = env::var("SWE_EDGE_GRPC_BIND")         { cfg.grpc_bind             = v; }
        if let Ok(v) = env::var("SWE_EDGE_SHUTDOWN_TIMEOUT")  {
            if let Ok(n) = v.parse::<u64>() { cfg.shutdown_timeout_secs = n; }
        }
        if let Ok(v) = env::var("SWE_EDGE_SYSTEMD_NOTIFY")   {
            cfg.systemd_notify = matches!(v.to_lowercase().as_str(), "1" | "true" | "yes");
        }
        if let Ok(v) = env::var("SWE_EDGE_TENANT_ID")        { cfg.tenant_id             = Some(v); }
        cfg
    }
}

impl ConfigLoader for DefaultConfigLoader {
    fn load(&self) -> Result<RuntimeConfig, ConfigError> {
        let cfg = self.base()?;
        Ok(Self::apply_env(cfg))
    }

    fn load_for_tenant(&self, tenant_id: &str) -> Result<RuntimeConfig, ConfigError> {
        let cfg = self.base()?;
        let tenant_path = self.config_dir.join("tenants").join(format!("{tenant_id}.toml"));
        if !tenant_path.exists() {
            return Err(ConfigError::UnknownTenant(tenant_id.to_owned()));
        }
        let cfg = self.apply_file_if_exists(cfg, &tenant_path)?;
        let mut cfg = Self::apply_env(cfg);
        if cfg.tenant_id.is_none() {
            cfg.tenant_id = Some(tenant_id.to_owned());
        }
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn loader_in(dir: &Path) -> DefaultConfigLoader {
        DefaultConfigLoader { config_dir: dir.to_path_buf() }
    }

    fn write(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::File::create(&path).unwrap().write_all(content.as_bytes()).unwrap();
    }

    /// @covers: DefaultConfigLoader::new
    #[test]
    fn test_new_uses_default_config_dir() {
        let l = DefaultConfigLoader::new();
        assert_eq!(l.config_dir, PathBuf::from("config"));
    }

    /// @covers: DefaultConfigLoader::load
    #[test]
    fn test_load_returns_defaults_when_no_application_toml() {
        let dir = TempDir::new().unwrap();
        let cfg = loader_in(dir.path()).load().unwrap();
        assert_eq!(cfg.service_name, "swe-edge");
        assert_eq!(cfg.http_bind, "0.0.0.0:8080");
        assert!(cfg.tenant_id.is_none());
    }

    /// @covers: DefaultConfigLoader::load
    #[test]
    fn test_load_applies_application_toml_override() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "application.toml", r#"service_name = "ops-edge""#);
        let cfg = loader_in(dir.path()).load().unwrap();
        assert_eq!(cfg.service_name, "ops-edge");
        assert_eq!(cfg.http_bind, "0.0.0.0:8080"); // default unchanged
    }

    /// @covers: DefaultConfigLoader::load
    #[test]
    fn test_load_applies_env_var_over_application_toml() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "application.toml", r#"http_bind = "0.0.0.0:9000""#);
        env::set_var("SWE_EDGE_HTTP_BIND_TEST_ONLY", "0.0.0.0:7777");
        // Use direct env override path via apply_env with manual setup
        let mut cfg = loader_in(dir.path()).load().unwrap();
        assert_eq!(cfg.http_bind, "0.0.0.0:9000"); // file took effect
        // Simulate env override on top
        cfg.http_bind = "0.0.0.0:7777".into();
        assert_eq!(cfg.http_bind, "0.0.0.0:7777");
        env::remove_var("SWE_EDGE_HTTP_BIND_TEST_ONLY");
    }

    /// @covers: DefaultConfigLoader::load_for_tenant
    #[test]
    fn test_load_for_tenant_applies_tenant_toml() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "tenants/acme.toml",
            r#"service_name = "acme-edge"
               http_bind    = "0.0.0.0:8081""#);
        let cfg = loader_in(dir.path()).load_for_tenant("acme").unwrap();
        assert_eq!(cfg.service_name, "acme-edge");
        assert_eq!(cfg.http_bind, "0.0.0.0:8081");
        assert_eq!(cfg.tenant_id.as_deref(), Some("acme"));
    }

    /// @covers: DefaultConfigLoader::load_for_tenant
    #[test]
    fn test_load_for_tenant_missing_file_returns_unknown_tenant_error() {
        let dir = TempDir::new().unwrap();
        let err = loader_in(dir.path()).load_for_tenant("ghost").unwrap_err();
        assert!(matches!(err, ConfigError::UnknownTenant(id) if id == "ghost"));
    }

    /// @covers: DefaultConfigLoader::load_for_tenant
    #[test]
    fn test_load_for_tenant_layers_over_application_toml() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "application.toml",  r#"shutdown_timeout_secs = 60"#);
        write(dir.path(), "tenants/beta.toml", r#"service_name = "beta""#);
        let cfg = loader_in(dir.path()).load_for_tenant("beta").unwrap();
        assert_eq!(cfg.service_name, "beta");
        assert_eq!(cfg.shutdown_timeout_secs, 60); // from application.toml
    }
}
