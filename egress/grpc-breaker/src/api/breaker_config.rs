//! gRPC circuit-breaker policy schema.
//!
//! Policy **values** live in TOML:
//! - crate-shipped baseline: `config/application.toml`
//! - consumer override: whatever TOML the binary loads and
//!   passes to [`GrpcBreakerConfig::from_config`].
//!
//! No `Default` impl — per the config-driven principle, policy
//! is data in a file, not literals in a source tree.

use std::time::Duration;

use serde::Deserialize;

use crate::api::error::Error;

/// gRPC circuit-breaker policy schema.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcBreakerConfig {
    /// Consecutive failures that trip the breaker open.
    pub failure_threshold: u32,

    /// Seconds to wait in Open state before the next request
    /// promotes to HalfOpen.
    pub cool_down_seconds: u64,

    /// Consecutive probe successes required in HalfOpen to
    /// close the breaker.
    pub half_open_probe_count: u32,
}

impl GrpcBreakerConfig {
    /// Parse a config from TOML text.
    pub fn from_config(toml_text: &str) -> Result<Self, Error> {
        let cfg: Self =
            toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Load the SWE-standard baseline.
    pub fn swe_default() -> Result<Self, Error> {
        Self::from_config(include_str!("../../config/application.toml"))
    }

    /// Cool-down as a [`Duration`].
    pub fn cool_down(&self) -> Duration {
        Duration::from_secs(self.cool_down_seconds)
    }

    fn validate(&self) -> Result<(), Error> {
        if self.failure_threshold == 0 {
            return Err(Error::InvalidConfig(
                "failure_threshold must be >= 1".into(),
            ));
        }
        if self.half_open_probe_count == 0 {
            return Err(Error::InvalidConfig(
                "half_open_probe_count must be >= 1".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: from_config
    #[test]
    fn test_from_config_parses_full_toml() {
        let toml = r#"
            failure_threshold = 5
            cool_down_seconds = 30
            half_open_probe_count = 1
        "#;
        let cfg = GrpcBreakerConfig::from_config(toml).expect("parses");
        assert_eq!(cfg.failure_threshold, 5);
        assert_eq!(cfg.cool_down_seconds, 30);
        assert_eq!(cfg.half_open_probe_count, 1);
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_unknown_key_is_error() {
        let toml = r#"
            failure_threshold = 5
            cool_down_seconds = 30
            half_open_probe_count = 1
            unknown = 99
        "#;
        let err = GrpcBreakerConfig::from_config(toml).unwrap_err();
        let s   = err.to_string();
        assert!(
            s.contains("unknown") || s.contains("unknown field"),
            "expected error to name unknown field, got: {s}",
        );
    }

    /// @covers: validate
    #[test]
    fn test_zero_threshold_is_invalid() {
        let toml = r#"
            failure_threshold = 0
            cool_down_seconds = 30
            half_open_probe_count = 1
        "#;
        let err = GrpcBreakerConfig::from_config(toml).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig(_)));
    }

    /// @covers: validate
    #[test]
    fn test_zero_probe_count_is_invalid() {
        let toml = r#"
            failure_threshold = 5
            cool_down_seconds = 30
            half_open_probe_count = 0
        "#;
        let err = GrpcBreakerConfig::from_config(toml).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig(_)));
    }

    /// @covers: swe_default
    #[test]
    fn test_swe_default_loads_crate_baseline() {
        let cfg = GrpcBreakerConfig::swe_default().expect("baseline parses");
        assert!(cfg.failure_threshold >= 1);
        assert!(cfg.half_open_probe_count >= 1);
    }

    /// @covers: cool_down
    #[test]
    fn test_cool_down_returns_duration_in_seconds() {
        let cfg = GrpcBreakerConfig::swe_default().unwrap();
        assert_eq!(cfg.cool_down(), Duration::from_secs(cfg.cool_down_seconds));
    }
}
