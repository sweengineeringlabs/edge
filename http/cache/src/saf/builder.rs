//! Public builder entry point.

use crate::api::cache_config::CacheConfig;
use crate::api::cache_layer::CacheLayer;
use crate::api::error::Error;

use crate::core::cache_layer as _;

/// Start configuring the cache with the SWE baseline loaded
/// from the crate-shipped `config/application.toml`.
pub fn builder() -> Result<Builder, Error> {
    let cfg = CacheConfig::swe_default()?;
    Ok(Builder::with_config(cfg))
}

pub use crate::api::builder::Builder;

impl Builder {
    /// Construct from a caller-supplied config.
    pub fn with_config(config: CacheConfig) -> Self {
        Self { config }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Finalize into the [`CacheLayer`].
    pub fn build(self) -> Result<CacheLayer, Error> {
        Ok(CacheLayer::new(self.config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: builder
    #[test]
    fn test_builder_loads_swe_default() {
        let b = builder().expect("baseline parses");
        assert!(b.config().max_entries > 0);
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_returns_cache_layer() {
        let layer = builder().expect("baseline").build().expect("build ok");
        let s = format!("{layer:?}");
        assert!(s.contains("CacheLayer"));
    }
}
