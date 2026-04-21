//! Public builder entry point.

use crate::api::retry_config::RetryConfig;
use crate::api::retry_layer::RetryLayer;
use crate::api::error::Error;

use crate::core::retry_layer as _;

/// Start configuring the retry middleware with the SWE baseline
/// loaded from the crate-shipped `config/application.toml`.
pub fn builder() -> Result<Builder, Error> {
    let cfg = RetryConfig::swe_default()?;
    Ok(Builder::with_config(cfg))
}

/// Builder handle.
#[derive(Debug)]
pub struct Builder {
    config: RetryConfig,
}

impl Builder {
    /// Construct from a caller-supplied config.
    pub fn with_config(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// Finalize into the [`RetryLayer`]. Takes config by value;
    /// the resulting middleware holds an `Arc<RetryConfig>` so
    /// it can be cloned cheaply across the middleware chain.
    pub fn build(self) -> Result<RetryLayer, Error> {
        Ok(RetryLayer::new(self.config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: builder
    #[test]
    fn test_builder_loads_swe_default() {
        let b = builder().expect("baseline parses");
        assert!(b.config().max_retries >= 1);
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_returns_retry_layer() {
        let layer = builder().expect("baseline").build().expect("build ok");
        let s = format!("{layer:?}");
        assert!(s.contains("RetryLayer"));
        assert!(s.contains("max_retries"));
    }
}
