//! gRPC retry policy schema.
//!
//! Policy **values** live in TOML:
//! - crate-shipped baseline: `config/application.toml`
//! - consumer override: whatever TOML the binary loads and
//!   passes to [`GrpcRetryConfig::from_config`].
//!
//! No `Default` impl — per the config-driven principle, policy
//! is data in a file, not literals in a source tree.

use std::time::Duration;

use serde::Deserialize;

use crate::api::error::Error;

/// gRPC retry policy schema.  Construct via
/// [`GrpcRetryConfig::from_config`] (custom TOML) or
/// [`GrpcRetryConfig::swe_default`] (crate baseline).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcRetryConfig {
    /// Maximum attempts per request (1 = no retry).
    pub max_attempts: u32,

    /// Delay before the first retry, in milliseconds.
    pub initial_backoff_ms: u64,

    /// Exponential backoff base (e.g. 2.0 → 100ms, 200ms, 400ms).
    pub backoff_multiplier: f64,

    /// Jitter as a fraction of the computed backoff (0.0 = none,
    /// 0.1 = up to 10% random delta).
    pub jitter_factor: f64,

    /// Upper bound on any single retry backoff in milliseconds.
    pub max_backoff_ms: u64,
}

impl GrpcRetryConfig {
    /// Parse a config from TOML text.
    ///
    /// Returns [`Error::ParseFailed`] when the text isn't valid
    /// TOML, when a required key is missing, or when an unknown
    /// key is present (`deny_unknown_fields` is set).  Returns
    /// [`Error::InvalidConfig`] when a value is out of range
    /// (e.g. `backoff_multiplier <= 0.0`).
    pub fn from_config(toml_text: &str) -> Result<Self, Error> {
        let cfg: Self =
            toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Load the SWE-standard baseline (the crate-shipped
    /// `config/application.toml`).  The file is embedded at
    /// build time via `include_str!`.
    pub fn swe_default() -> Result<Self, Error> {
        Self::from_config(include_str!("../../config/application.toml"))
    }

    /// Initial backoff as a [`Duration`].
    pub fn initial_backoff(&self) -> Duration {
        Duration::from_millis(self.initial_backoff_ms)
    }

    /// Maximum single-retry backoff as a [`Duration`].
    pub fn max_backoff(&self) -> Duration {
        Duration::from_millis(self.max_backoff_ms)
    }

    fn validate(&self) -> Result<(), Error> {
        if self.max_attempts == 0 {
            return Err(Error::InvalidConfig(
                "max_attempts must be >= 1".into(),
            ));
        }
        if self.backoff_multiplier <= 0.0 || !self.backoff_multiplier.is_finite() {
            return Err(Error::InvalidConfig(
                "backoff_multiplier must be a positive finite number".into(),
            ));
        }
        if !(0.0..=1.0).contains(&self.jitter_factor) || !self.jitter_factor.is_finite() {
            return Err(Error::InvalidConfig(
                "jitter_factor must be in [0.0, 1.0]".into(),
            ));
        }
        if self.max_backoff_ms < self.initial_backoff_ms {
            return Err(Error::InvalidConfig(
                "max_backoff_ms must be >= initial_backoff_ms".into(),
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
            max_attempts = 5
            initial_backoff_ms = 100
            backoff_multiplier = 2.0
            jitter_factor = 0.1
            max_backoff_ms = 5000
        "#;
        let cfg = GrpcRetryConfig::from_config(toml).expect("parses");
        assert_eq!(cfg.max_attempts, 5);
        assert_eq!(cfg.initial_backoff_ms, 100);
        assert_eq!(cfg.backoff_multiplier, 2.0);
        assert_eq!(cfg.jitter_factor, 0.1);
        assert_eq!(cfg.max_backoff_ms, 5000);
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_missing_key_is_error() {
        let toml = r#"
            max_attempts = 5
            initial_backoff_ms = 100
            backoff_multiplier = 2.0
            # missing jitter_factor
            max_backoff_ms = 5000
        "#;
        let err = GrpcRetryConfig::from_config(toml).unwrap_err();
        let s   = err.to_string();
        assert!(
            s.contains("jitter_factor") || s.contains("missing field"),
            "expected error to name the missing field, got: {s}",
        );
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_unknown_key_is_error() {
        let toml = r#"
            max_attempts = 5
            initial_backoff_ms = 100
            backoff_multiplier = 2.0
            jitter_factor = 0.1
            max_backoff_ms = 5000
            unknown_knob = 42
        "#;
        let err = GrpcRetryConfig::from_config(toml).unwrap_err();
        let s   = err.to_string();
        assert!(
            s.contains("unknown_knob") || s.contains("unknown field"),
            "expected error to name unknown field, got: {s}",
        );
    }

    /// @covers: validate
    #[test]
    fn test_zero_max_attempts_is_invalid() {
        let toml = r#"
            max_attempts = 0
            initial_backoff_ms = 100
            backoff_multiplier = 2.0
            jitter_factor = 0.1
            max_backoff_ms = 5000
        "#;
        let err = GrpcRetryConfig::from_config(toml).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig(_)));
    }

    /// @covers: validate
    #[test]
    fn test_negative_multiplier_is_invalid() {
        let toml = r#"
            max_attempts = 5
            initial_backoff_ms = 100
            backoff_multiplier = -1.0
            jitter_factor = 0.1
            max_backoff_ms = 5000
        "#;
        let err = GrpcRetryConfig::from_config(toml).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig(_)));
    }

    /// @covers: validate
    #[test]
    fn test_jitter_above_one_is_invalid() {
        let toml = r#"
            max_attempts = 5
            initial_backoff_ms = 100
            backoff_multiplier = 2.0
            jitter_factor = 1.5
            max_backoff_ms = 5000
        "#;
        let err = GrpcRetryConfig::from_config(toml).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig(_)));
    }

    /// @covers: validate
    #[test]
    fn test_max_backoff_smaller_than_initial_is_invalid() {
        let toml = r#"
            max_attempts = 5
            initial_backoff_ms = 5000
            backoff_multiplier = 2.0
            jitter_factor = 0.1
            max_backoff_ms = 100
        "#;
        let err = GrpcRetryConfig::from_config(toml).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig(_)));
    }

    /// @covers: swe_default
    #[test]
    fn test_swe_default_loads_crate_baseline() {
        let cfg = GrpcRetryConfig::swe_default().expect("baseline must parse");
        assert!(cfg.max_attempts >= 1);
    }

    /// @covers: initial_backoff
    #[test]
    fn test_initial_backoff_returns_duration_in_milliseconds() {
        let cfg = GrpcRetryConfig::swe_default().unwrap();
        assert_eq!(
            cfg.initial_backoff(),
            Duration::from_millis(cfg.initial_backoff_ms),
        );
    }

    /// @covers: max_backoff
    #[test]
    fn test_max_backoff_returns_duration_in_milliseconds() {
        let cfg = GrpcRetryConfig::swe_default().unwrap();
        assert_eq!(
            cfg.max_backoff(),
            Duration::from_millis(cfg.max_backoff_ms),
        );
    }
}
