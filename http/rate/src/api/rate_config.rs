//! Client-side rate-limiter policy schema. Values live in
//! `config/application.toml`.

use serde::Deserialize;

use crate::api::error::Error;

/// Rate-limiter (token-bucket) policy schema.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateConfig {
    /// Sustained refill rate, tokens per second.
    pub tokens_per_second: u32,
    /// Bucket capacity (burst tolerance).
    pub burst_capacity: u32,
    /// Per-host bucketing (false = single global bucket).
    pub per_host: bool,
}

impl RateConfig {
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
            tokens_per_second = 50
            burst_capacity = 100
            per_host = false
        "#;
        let cfg = RateConfig::from_config(toml).unwrap();
        assert_eq!(cfg.tokens_per_second, 50);
        assert!(!cfg.per_host);
    }

    /// @covers: swe_default
    #[test]
    fn test_swe_default_loads_crate_baseline() {
        let cfg = RateConfig::swe_default().unwrap();
        assert!(cfg.tokens_per_second >= 1);
    }
}
