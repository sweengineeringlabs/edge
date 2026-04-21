//! Public builder entry point.

use crate::api::breaker_config::BreakerConfig;
use crate::api::breaker_layer::BreakerLayer;
use crate::api::error::Error;

use crate::core::breaker_layer as _;

/// Start configuring the breaker with the SWE baseline
/// loaded from the crate-shipped `config/application.toml`.
pub fn builder() -> Result<Builder, Error> {
    let cfg = BreakerConfig::swe_default()?;
    Ok(Builder::with_config(cfg))
}

/// Builder handle.
#[derive(Debug)]
pub struct Builder {
    config: BreakerConfig,
}

impl Builder {
    /// Construct from a caller-supplied config.
    pub fn with_config(config: BreakerConfig) -> Self {
        Self { config }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &BreakerConfig {
        &self.config
    }

    /// Finalize into the [`BreakerLayer`]. The returned layer
    /// implements `reqwest_middleware::Middleware` and carries
    /// its own per-host state cache (bounded moka cache).
    pub fn build(self) -> Result<BreakerLayer, Error> {
        Ok(BreakerLayer::new(self.config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: builder
    #[test]
    fn test_builder_loads_swe_default() {
        let b = builder().expect("baseline parses");
        assert!(b.config().failure_threshold >= 1);
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_returns_breaker_layer() {
        let layer = builder().expect("baseline").build().expect("build ok");
        let s = format!("{layer:?}");
        assert!(s.contains("BreakerLayer"));
    }
}
