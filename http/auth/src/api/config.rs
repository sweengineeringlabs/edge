//! Auth policy schema. Values live in `config/default.toml`.
//!
//! **Credential resolution is env-var-only.** The config stores
//! the NAME of the env var holding the credential, not the
//! credential itself. This is enforced at the schema level —
//! there are no `token: String` or `password: String` fields on
//! any variant.

use serde::Deserialize;

use crate::api::error::Error;

/// Auth policy schema. A tagged enum on the `kind` field so
/// config like `kind = "bearer"; token_env = "..."` deserializes
/// into the right variant.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthConfig {
    /// Pass-through — middleware attached but attaches no
    /// credential. The baseline; consumers must explicitly
    /// switch to a real scheme to activate auth.
    None,

    /// `Authorization: Bearer <token>`. Token is read from the
    /// named env var at config-load time.
    Bearer {
        /// Name of the env var holding the bearer token.
        token_env: String,
    },

    /// `Authorization: Basic base64(user:pass)`. User + password
    /// are read from the named env vars at config-load time.
    Basic {
        /// Env var holding the username.
        user_env: String,
        /// Env var holding the password.
        pass_env: String,
    },

    /// Custom credential header (e.g. `x-api-key`,
    /// `x-goog-api-key`). For APIs that use non-standard auth
    /// headers instead of `Authorization`.
    Header {
        /// The HTTP header name to set (lowercase recommended
        /// for clarity; case-insensitive on the wire).
        name: String,
        /// Env var holding the credential value.
        value_env: String,
    },
}

impl AuthConfig {
    /// Parse from TOML text.
    pub fn from_config(toml_text: &str) -> Result<Self, Error> {
        toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))
    }

    /// Load the crate-shipped SWE baseline (`kind = "none"`).
    pub fn swe_default() -> Result<Self, Error> {
        Self::from_config(include_str!("../../config/default.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_none() {
        let cfg = AuthConfig::from_config(r#"kind = "none""#).unwrap();
        assert!(matches!(cfg, AuthConfig::None));
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_bearer() {
        let cfg = AuthConfig::from_config(
            r#"
                kind = "bearer"
                token_env = "EDGE_API_TOKEN"
            "#,
        )
        .unwrap();
        match cfg {
            AuthConfig::Bearer { token_env } => assert_eq!(token_env, "EDGE_API_TOKEN"),
            other => panic!("expected Bearer, got {other:?}"),
        }
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_basic() {
        let cfg = AuthConfig::from_config(
            r#"
                kind = "basic"
                user_env = "U"
                pass_env = "P"
            "#,
        )
        .unwrap();
        assert!(matches!(cfg, AuthConfig::Basic { .. }));
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_header() {
        let cfg = AuthConfig::from_config(
            r#"
                kind = "header"
                name = "x-api-key"
                value_env = "EDGE_API_KEY"
            "#,
        )
        .unwrap();
        match cfg {
            AuthConfig::Header { name, value_env } => {
                assert_eq!(name, "x-api-key");
                assert_eq!(value_env, "EDGE_API_KEY");
            }
            other => panic!("expected Header, got {other:?}"),
        }
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_unknown_kind_is_error() {
        let err = AuthConfig::from_config(r#"kind = "digest""#).unwrap_err();
        let s = err.to_string();
        // serde tagged-enum rejection names the variant list.
        assert!(s.contains("digest") || s.contains("variant"));
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_bearer_rejects_inline_token() {
        // Crucial security property: a bearer block with
        // `token = "raw_secret"` must NOT deserialize. Only
        // `token_env` (name-of-env-var) is a valid field.
        let err = AuthConfig::from_config(
            r#"
                kind = "bearer"
                token = "dont_commit_this"
            "#,
        )
        .unwrap_err();
        let s = err.to_string();
        assert!(
            s.contains("unknown") || s.contains("token_env") || s.contains("token"),
            "expected rejection of inline `token` field, got: {s}"
        );
    }

    /// @covers: swe_default
    #[test]
    fn test_swe_default_is_none_pass_through() {
        let cfg = AuthConfig::swe_default().unwrap();
        assert!(matches!(cfg, AuthConfig::None));
    }
}
