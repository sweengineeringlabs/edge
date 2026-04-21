//! Cache policy schema — the struct layout, nothing else.
//!
//! Policy **values** live in TOML:
//! - crate-shipped baseline: `config/default.toml`
//! - workspace override: `edge/http/main/config/application.toml` under `[cache]`
//! - consumer override: whatever TOML the binary loads
//!
//! No `Default` impl with literal numbers — per the
//! config-driven principle, policy is data in a file, not code
//! in a source tree.

use serde::Deserialize;

use crate::api::error::Error;

/// HTTP cache policy schema.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheConfig {
    /// Fallback TTL when the upstream response lacks
    /// Cache-Control max-age.
    pub default_ttl_seconds: u64,
    /// Maximum entries in the in-memory store (LRU eviction).
    pub max_entries: u64,
    /// Honor upstream Cache-Control headers (no-store / max-age).
    pub respect_cache_control: bool,
    /// Cache responses marked `Cache-Control: private`.
    pub cache_private: bool,
}

impl CacheConfig {
    /// Parse from TOML text.
    pub fn from_config(toml_text: &str) -> Result<Self, Error> {
        toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))
    }

    /// Load the crate-shipped SWE baseline.
    pub fn swe_default() -> Result<Self, Error> {
        Self::from_config(include_str!("../../config/default.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_full_toml() {
        let toml = r#"
            default_ttl_seconds = 60
            max_entries = 100
            respect_cache_control = true
            cache_private = false
        "#;
        let cfg = CacheConfig::from_config(toml).unwrap();
        assert_eq!(cfg.default_ttl_seconds, 60);
        assert!(cfg.respect_cache_control);
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_unknown_key_is_error() {
        let toml = r#"
            default_ttl_seconds = 60
            max_entries = 100
            respect_cache_control = true
            cache_private = false
            weird_knob = 42
        "#;
        let err = CacheConfig::from_config(toml).unwrap_err();
        let s = err.to_string();
        assert!(s.contains("weird_knob") || s.contains("unknown"));
    }

    /// @covers: swe_default
    #[test]
    fn test_swe_default_loads_crate_baseline() {
        let cfg = CacheConfig::swe_default().unwrap();
        assert!(cfg.max_entries > 0);
    }
}
