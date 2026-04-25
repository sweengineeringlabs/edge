//! SAF builder entry point for the main composition crate.

use swe_edge_http_auth::AuthConfig;

pub use crate::api::builder::Builder;
pub use crate::api::error::Error;
pub use crate::api::stack_config::StackConfig;

/// Start configuring the middleware stack with SWE defaults.
pub fn builder() -> Result<Builder, Error> {
    let config = StackConfig {
        auth: AuthConfig::None,
    };
    Ok(Builder { config })
}

impl Builder {
    /// Construct from a caller-supplied aggregate config.
    pub fn with_config(config: StackConfig) -> Self {
        Self { config }
    }

    /// Borrow the current aggregate config.
    pub fn config(&self) -> &StackConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swe_edge_http_auth::AuthConfig;

    #[test]
    fn test_builder_fn_produces_builder_with_none_auth() {
        let b = builder().expect("swe default must succeed");
        assert!(matches!(b.config().auth, AuthConfig::None));
    }

    #[test]
    fn test_with_config_stores_supplied_config() {
        let cfg = StackConfig { auth: AuthConfig::None };
        let b = Builder::with_config(cfg);
        assert!(matches!(b.config().auth, AuthConfig::None));
    }
}
