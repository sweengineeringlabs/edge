//! Config error type and partial-override deserialization.

use serde::Deserialize;
use thiserror::Error;

use crate::api::types::RuntimeConfig;

/// Errors that can occur when loading daemon configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The TOML text could not be parsed.
    #[error("config parse error: {0}")]
    Parse(String),
    /// A required config file could not be read.
    #[error("config io error: {0}")]
    Io(String),
    /// The requested tenant config file does not exist.
    #[error("unknown tenant: '{0}'")]
    UnknownTenant(String),
    /// The tenant ID contains characters that are not permitted.
    ///
    /// Only `[a-zA-Z0-9_-]` are allowed; `.`, `/`, `\`, and NUL are rejected
    /// to prevent path traversal attacks.
    #[error("invalid tenant id: '{0}' — only [a-zA-Z0-9_-] are allowed")]
    InvalidTenantId(String),
    /// An environment variable was set but its value could not be parsed.
    #[error("invalid env var: {0}")]
    BadEnvVar(String),
}

/// A partial `RuntimeConfig` — all fields optional so any
/// subset of keys can be present in a TOML overlay file.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct ConfigOverride {
    pub(crate) service_name:          Option<String>,
    pub(crate) http_bind:             Option<String>,
    pub(crate) grpc_bind:             Option<String>,
    pub(crate) shutdown_timeout_secs: Option<u64>,
    pub(crate) systemd_notify:        Option<bool>,
    pub(crate) tenant_id:             Option<String>,
}

impl ConfigOverride {
    /// Parse a TOML string into a partial override.
    pub(crate) fn from_str(s: &str) -> Result<Self, ConfigError> {
        toml::from_str(s).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Apply this override onto a base config, returning the merged result.
    pub(crate) fn apply_to(self, mut base: RuntimeConfig) -> RuntimeConfig {
        if let Some(v) = self.service_name          { base.service_name          = v; }
        if let Some(v) = self.http_bind             { base.http_bind             = v; }
        if let Some(v) = self.grpc_bind             { base.grpc_bind             = v; }
        if let Some(v) = self.shutdown_timeout_secs { base.shutdown_timeout_secs = v; }
        if let Some(v) = self.systemd_notify        { base.systemd_notify        = v; }
        if let Some(v) = self.tenant_id             { base.tenant_id             = Some(v); }
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: ConfigOverride::from_str
    #[test]
    fn test_from_str_parses_partial_toml() {
        let o = ConfigOverride::from_str(r#"http_bind = "127.0.0.1:9090""#).unwrap();
        assert_eq!(o.http_bind.as_deref(), Some("127.0.0.1:9090"));
        assert!(o.service_name.is_none());
    }

    /// @covers: ConfigOverride::from_str
    #[test]
    fn test_from_str_empty_toml_gives_all_none() {
        let o = ConfigOverride::from_str("").unwrap();
        assert!(o.service_name.is_none());
        assert!(o.http_bind.is_none());
    }

    /// @covers: ConfigOverride::from_str
    #[test]
    fn test_from_str_invalid_toml_returns_parse_error() {
        let err = ConfigOverride::from_str("not = [valid toml").unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)));
    }

    /// @covers: ConfigOverride::apply_to
    #[test]
    fn test_apply_to_overrides_set_fields_only() {
        let base = RuntimeConfig::default();
        let o = ConfigOverride::from_str(r#"service_name = "acme""#).unwrap();
        let merged = o.apply_to(base);
        assert_eq!(merged.service_name, "acme");
        assert_eq!(merged.http_bind, "0.0.0.0:8080"); // unchanged
    }

    /// @covers: ConfigOverride::apply_to
    #[test]
    fn test_apply_to_sets_tenant_id() {
        let base = RuntimeConfig::default();
        let o = ConfigOverride::from_str(r#"tenant_id = "t-001""#).unwrap();
        let merged = o.apply_to(base);
        assert_eq!(merged.tenant_id.as_deref(), Some("t-001"));
    }

    /// @covers: ConfigError
    #[test]
    fn test_config_error_display_parse() {
        let e = ConfigError::Parse("bad toml".into());
        assert!(e.to_string().contains("parse error"));
    }

    /// @covers: ConfigError
    #[test]
    fn test_config_error_display_unknown_tenant() {
        let e = ConfigError::UnknownTenant("ghost".into());
        assert!(e.to_string().contains("ghost"));
    }

    /// @covers: ConfigError::InvalidTenantId
    #[test]
    fn test_config_error_display_invalid_tenant_id() {
        let e = ConfigError::InvalidTenantId("../../etc".into());
        assert!(e.to_string().contains("../../etc"));
        assert!(e.to_string().contains("[a-zA-Z0-9_-]"));
    }

    /// @covers: ConfigError::BadEnvVar
    #[test]
    fn test_config_error_display_bad_env_var() {
        let e = ConfigError::BadEnvVar("SWE_EDGE_SHUTDOWN_TIMEOUT=\"abc\": expected a non-negative integer".into());
        assert!(e.to_string().contains("SWE_EDGE_SHUTDOWN_TIMEOUT"));
    }
}
