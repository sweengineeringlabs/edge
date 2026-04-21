//! Public builder entry point.
//!
//! Consumers construct a [`CacheConfig`] — usually via
//! [`CacheConfig::swe_default`] or [`CacheConfig::from_config`]
//! with their own TOML — then hand it to the builder. Policy
//! lives in config files, not in chained method calls.

use crate::api::cache_config::CacheConfig;
use crate::api::error::Error;

/// Start configuring the middleware with the SWE baseline loaded
/// from the crate-shipped `config/default.toml`. For non-default
/// policy, construct a [`CacheConfig`] directly and use
/// [`Builder::with_config`].
pub fn builder() -> Result<Builder, Error> {
    let cfg = CacheConfig::swe_default()?;
    Ok(Builder { config: cfg })
}

/// Builder handle. Opaque — knobs live on the config.
#[derive(Debug)]
pub struct Builder {
    config: CacheConfig,
}

impl Builder {
    /// Construct from a caller-supplied config.
    pub fn with_config(config: CacheConfig) -> Self {
        Self { config }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Finalize into the middleware layer. Scaffold phase:
    /// returns NotImplemented until the real impl lands.
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
        let _b = builder().expect("baseline must parse");
    }

    /// @covers: Builder::with_config
    #[test]
    fn test_with_config_holds_baseline_policy() {
        // Reuse the baseline as a valid config for this test —
        // the point is that the type round-trips through the
        // builder, not that we supply novel values.
        let cfg = CacheConfig::swe_default().expect("baseline parses");
        let _b = Builder::with_config(cfg);
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_returns_not_implemented_during_scaffold_phase() {
        let b = builder().expect("baseline parses");
        let err = b.build().unwrap_err();
        assert!(matches!(err, Error::NotImplemented(_)));
    }
}
