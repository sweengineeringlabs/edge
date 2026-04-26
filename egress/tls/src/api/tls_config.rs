//! TLS identity policy schema. Values live in
//! `config/application.toml`. No `Default` impl with literal
//! values — per the config-driven principle.

use serde::Deserialize;

use crate::api::error::Error;

/// TLS client-identity schema. Tagged enum on `kind` so
/// `kind = "pkcs12"; path = "..."; password_env = "..."`
/// deserializes into the right variant.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TlsConfig {
    /// Pass-through — no client cert attached. Baseline.
    None,

    /// PKCS#12 bundle (.p12 / .pfx) with cert + private key.
    Pkcs12 {
        /// Path to the .p12 / .pfx file.
        path: String,
        /// Env var holding the decryption password. Optional —
        /// omit for passwordless bundles.
        #[serde(default)]
        password_env: Option<String>,
    },

    /// Combined PEM file containing cert chain + private key.
    Pem {
        /// Path to the .pem file.
        path: String,
    },
}

impl TlsConfig {
    /// Parse from TOML text.
    pub fn from_config(toml_text: &str) -> Result<Self, Error> {
        toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))
    }

    /// Load the crate-shipped SWE baseline (`kind = "none"`).
    pub fn swe_default() -> Result<Self, Error> {
        Self::from_config(include_str!("../../config/application.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_none() {
        let cfg = TlsConfig::from_config(r#"kind = "none""#).unwrap();
        assert!(matches!(cfg, TlsConfig::None));
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_pkcs12_with_password() {
        let cfg = TlsConfig::from_config(
            r#"
                kind = "pkcs12"
                path = "/etc/client.p12"
                password_env = "EDGE_TLS_PASSWORD"
            "#,
        )
        .unwrap();
        match cfg {
            TlsConfig::Pkcs12 { path, password_env } => {
                assert_eq!(path, "/etc/client.p12");
                assert_eq!(password_env.as_deref(), Some("EDGE_TLS_PASSWORD"));
            }
            other => panic!("expected Pkcs12, got {other:?}"),
        }
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_pkcs12_without_password() {
        let cfg = TlsConfig::from_config(
            r#"
                kind = "pkcs12"
                path = "/etc/client.p12"
            "#,
        )
        .unwrap();
        match cfg {
            TlsConfig::Pkcs12 { path, password_env } => {
                assert_eq!(path, "/etc/client.p12");
                assert!(password_env.is_none());
            }
            other => panic!("expected Pkcs12, got {other:?}"),
        }
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_pem() {
        let cfg = TlsConfig::from_config(
            r#"
                kind = "pem"
                path = "/etc/client-combined.pem"
            "#,
        )
        .unwrap();
        match cfg {
            TlsConfig::Pem { path } => assert_eq!(path, "/etc/client-combined.pem"),
            other => panic!("expected Pem, got {other:?}"),
        }
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_rejects_inline_password() {
        // Security property: inline `password = "raw"` must NOT
        // deserialize — only password_env (env-var-name) is
        // accepted.
        let err = TlsConfig::from_config(
            r#"
                kind = "pkcs12"
                path = "/etc/client.p12"
                password = "secret"
            "#,
        )
        .unwrap_err();
        let s = err.to_string();
        assert!(
            s.contains("unknown") || s.contains("password_env") || s.contains("password"),
            "expected rejection of inline `password` field, got: {s}"
        );
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_rejects_unknown_kind() {
        let err = TlsConfig::from_config(r#"kind = "jks""#).unwrap_err();
        assert!(err.to_string().contains("jks") || err.to_string().contains("variant"));
    }

    /// @covers: swe_default
    #[test]
    fn test_swe_default_is_none() {
        let cfg = TlsConfig::swe_default().unwrap();
        assert!(matches!(cfg, TlsConfig::None));
    }
}
