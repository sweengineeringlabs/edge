//! Public builder + factory.

use swe_edge_egress_grpc::GrpcOutbound;

use crate::api::breaker_client::GrpcBreakerClient;
use crate::api::breaker_config::GrpcBreakerConfig;
use crate::api::error::Error;

/// Start configuring the breaker with the SWE baseline loaded
/// from `config/application.toml`.
pub fn builder() -> Result<Builder, Error> {
    let cfg = GrpcBreakerConfig::swe_default()?;
    Ok(Builder::with_config(cfg))
}

/// One-shot factory: wrap `inner` with the SWE baseline policy.
pub fn create_breaker_client<T: GrpcOutbound + Send + Sync + 'static>(
    inner: T,
) -> Result<GrpcBreakerClient<T>, Error> {
    let cfg = GrpcBreakerConfig::swe_default()?;
    Ok(GrpcBreakerClient::new(inner, cfg))
}

pub use crate::api::builder::Builder;

impl Builder {
    /// Construct from a caller-supplied config.
    pub fn with_config(config: GrpcBreakerConfig) -> Self {
        Self { config }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &GrpcBreakerConfig {
        &self.config
    }

    /// Wrap `inner` to produce a [`GrpcBreakerClient`].
    pub fn wrap<T: GrpcOutbound + Send + Sync + 'static>(
        self,
        inner: T,
    ) -> GrpcBreakerClient<T> {
        GrpcBreakerClient::new(inner, self.config)
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

    /// @covers: Builder::with_config
    #[test]
    fn test_with_config_stores_provided_config() {
        let cfg = GrpcBreakerConfig::from_config(
            r#"
                failure_threshold = 7
                cool_down_seconds = 60
                half_open_probe_count = 3
            "#,
        )
        .unwrap();
        let b = Builder::with_config(cfg);
        assert_eq!(b.config().failure_threshold, 7);
        assert_eq!(b.config().cool_down_seconds, 60);
        assert_eq!(b.config().half_open_probe_count, 3);
    }
}
