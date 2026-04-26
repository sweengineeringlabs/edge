//! Circuit-breaker policy schema. Values live in
//! `config/application.toml`.

use serde::Deserialize;

use crate::api::error::Error;

/// Circuit-breaker policy schema.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BreakerConfig {
    /// Consecutive failures that trip the breaker open.
    pub failure_threshold: u32,
    /// Seconds to wait after opening before probing.
    pub half_open_after_seconds: u64,
    /// Consecutive probe successes required to close.
    pub reset_after_successes: u32,
    /// Response statuses counted as failures.
    pub failure_statuses: Vec<u16>,
}

impl BreakerConfig {
    /// Parse from TOML text.
    pub(crate) fn from_config(toml_text: &str) -> Result<Self, Error> {
        toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))
    }

    /// Load the crate-shipped SWE baseline.
    pub(crate) fn swe_default() -> Result<Self, Error> {
        Self::from_config(include_str!("../../config/application.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_full_toml() {
        let toml = r#"
            failure_threshold = 10
            half_open_after_seconds = 60
            reset_after_successes = 5
            failure_statuses = [500, 503]
        "#;
        let cfg = BreakerConfig::from_config(toml).unwrap();
        assert_eq!(cfg.failure_threshold, 10);
    }

    /// @covers: swe_default
    #[test]
    fn test_swe_default_loads_crate_baseline() {
        let cfg = BreakerConfig::swe_default().unwrap();
        assert!(cfg.failure_threshold >= 1);
    }
}
