//! Public builder entry point.
//!
//! Consumers construct a [`RetryConfig`] — usually by calling
//! [`RetryConfig::swe_default`] or [`RetryConfig::from_config`]
//! with their own TOML — then hand it to the builder. The
//! builder doesn't expose fluent `.max_retries(..)` overrides on
//! purpose: policy lives in config files, not chained method
//! calls. Last-mile programmatic tweaks mutate the `RetryConfig`
//! before passing it in.

use crate::api::retry_config::RetryConfig;
use crate::api::error::Error;

/// Start configuring the retry middleware with the SWE baseline
/// loaded from the crate-shipped `config/default.toml`.
///
/// Returns an error only if the baseline file fails to parse —
/// the crate's own tests lock that down, so in practice this
/// yields `Ok`.
///
/// For any non-default policy, construct a [`RetryConfig`]
/// directly (e.g. via [`RetryConfig::from_config`] on consumer
/// TOML) and use [`Builder::with_config`].
pub fn builder() -> Result<Builder, Error> {
    let cfg = RetryConfig::swe_default()?;
    Ok(Builder { config: cfg })
}

/// Builder handle for the retry middleware. Opaque by design —
/// all policy knobs are on the [`RetryConfig`] inside.
#[derive(Debug)]
pub struct Builder {
    config: RetryConfig,
}

impl Builder {
    /// Construct from a caller-supplied [`RetryConfig`]. Use this
    /// when loading policy from a non-default TOML source
    /// (workspace application.toml, consumer config file, etc.).
    pub fn with_config(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Borrow the current policy. Useful for assertions in
    /// scenario tests and for composing overrides.
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// Finalize into the middleware layer. Scaffold phase:
    /// returns [`Error::NotImplemented`]. When the real impl
    /// lands, this produces a `reqwest_middleware::Middleware`
    /// layer honoring the config.
    pub fn build(self) -> Result<(), Error> {
        Err(Error::NotImplemented("builder"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: builder
    #[test]
    fn test_builder_loads_swe_default_config() {
        let b = builder().expect("baseline must parse");
        // max_retries is a required field in config/default.toml;
        // if the baseline omitted it, from_config would have
        // returned Err above — so the value is guaranteed > 0.
        assert!(b.config().max_retries >= 1);
    }

    /// @covers: Builder::with_config
    #[test]
    fn test_with_config_holds_supplied_policy() {
        let cfg = RetryConfig::from_config(
            r#"
                max_retries = 99
                initial_interval_ms = 1
                max_interval_ms = 2
                multiplier = 1.0
                retryable_statuses = [429]
                retryable_methods = ["GET"]
            "#,
        )
        .unwrap();
        let b = Builder::with_config(cfg);
        assert_eq!(b.config().max_retries, 99);
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_returns_not_implemented_during_scaffold_phase() {
        let b = builder().expect("baseline parses");
        let err = b.build().unwrap_err();
        assert!(matches!(err, Error::NotImplemented(_)));
    }
}
