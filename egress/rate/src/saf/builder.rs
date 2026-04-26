//! Public builder entry point.

use crate::api::error::Error;
use crate::api::rate_config::RateConfig;
use crate::api::rate_layer::RateLayer;


/// Start configuring the rate limiter with the SWE baseline
/// loaded from the crate-shipped `config/application.toml`.
pub fn builder() -> Result<Builder, Error> {
    let cfg = RateConfig::swe_default()?;
    Ok(Builder::with_config(cfg))
}

pub use crate::api::builder::Builder;

impl Builder {
    /// Construct from a caller-supplied config.
    pub fn with_config(config: RateConfig) -> Self {
        Self { config }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &RateConfig {
        &self.config
    }

    /// Finalize into the [`RateLayer`].
    pub fn build(self) -> Result<RateLayer, Error> {
        Ok(RateLayer::new(self.config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: builder
    #[test]
    fn test_builder_loads_swe_default() {
        let b = builder().expect("baseline parses");
        assert!(b.config().tokens_per_second >= 1);
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_returns_rate_layer() {
        let layer = builder().expect("baseline").build().expect("build ok");
        let s = format!("{layer:?}");
        assert!(s.contains("RateLayer"));
    }
}
