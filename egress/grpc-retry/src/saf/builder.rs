//! Public builder + factory.

use swe_edge_egress_grpc::GrpcOutbound;

use crate::api::error::Error;
use crate::api::retry_client::GrpcRetryClient;
use crate::api::retry_config::GrpcRetryConfig;

/// Start configuring the retry decorator with the SWE baseline
/// loaded from the crate-shipped `config/application.toml`.
pub fn builder() -> Result<Builder, Error> {
    let cfg = GrpcRetryConfig::swe_default()?;
    Ok(Builder::with_config(cfg))
}

/// One-shot factory: wrap `inner` with the SWE baseline retry
/// policy.  Equivalent to `builder()?.wrap(inner)`.
///
/// Use [`builder`] when you need to override config before
/// wrapping; use this when the defaults are right.
pub fn create_retry_client<T: GrpcOutbound + Send + Sync + 'static>(
    inner: T,
) -> Result<GrpcRetryClient<T>, Error> {
    let cfg = GrpcRetryConfig::swe_default()?;
    Ok(GrpcRetryClient::new(inner, cfg))
}

pub use crate::api::builder::Builder;

impl Builder {
    /// Construct from a caller-supplied config.
    pub fn with_config(config: GrpcRetryConfig) -> Self {
        Self { config }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &GrpcRetryConfig {
        &self.config
    }

    /// Wrap `inner` to produce a [`GrpcRetryClient`].
    pub fn wrap<T: GrpcOutbound + Send + Sync + 'static>(
        self,
        inner: T,
    ) -> GrpcRetryClient<T> {
        GrpcRetryClient::new(inner, self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: builder
    #[test]
    fn test_builder_loads_swe_default() {
        let b = builder().expect("baseline parses");
        assert!(b.config().max_attempts >= 1);
    }

    /// @covers: Builder::with_config
    #[test]
    fn test_with_config_stores_provided_config() {
        let cfg = GrpcRetryConfig::from_config(
            r#"
                max_attempts = 7
                initial_backoff_ms = 50
                backoff_multiplier = 1.5
                jitter_factor = 0.2
                max_backoff_ms = 2000
            "#,
        )
        .unwrap();
        let b = Builder::with_config(cfg);
        assert_eq!(b.config().max_attempts, 7);
        assert_eq!(b.config().initial_backoff_ms, 50);
    }
}
