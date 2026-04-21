//! Public builder entry point.

use std::sync::Arc;

use crate::api::error::Error;
use crate::api::tls_config::TlsConfig;
use crate::api::tls_layer::TlsLayer;

use crate::core::identity::build_provider;
use crate::core::tls_layer as _;

/// Start configuring the TLS layer with the SWE baseline
/// (`kind = "none"` — pass-through).
pub fn builder() -> Result<Builder, Error> {
    let cfg = TlsConfig::swe_default()?;
    Ok(Builder::with_config(cfg))
}

/// Builder handle.
pub struct Builder {
    config: TlsConfig,
}

impl std::fmt::Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("config", &self.config)
            .finish()
    }
}

impl Builder {
    /// Construct from a caller-supplied config.
    pub fn with_config(config: TlsConfig) -> Self {
        Self { config }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &TlsConfig {
        &self.config
    }

    /// Finalize into the [`TlsLayer`]. Resolves file paths +
    /// env-var-backed passwords NOW — missing files or unset
    /// passwords fail startup rather than first request.
    pub fn build(self) -> Result<TlsLayer, Error> {
        let provider = build_provider(&self.config)?;
        Ok(TlsLayer::new(Arc::from(provider)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: builder
    #[test]
    fn test_builder_loads_swe_default_none() {
        let b = builder().expect("baseline parses");
        assert!(matches!(b.config(), TlsConfig::None));
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_with_none_config_returns_pass_through_layer() {
        let layer = builder().expect("baseline").build().expect("build ok");
        let s = format!("{layer:?}");
        assert!(s.contains("noop"));
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_with_missing_pem_file_fails_fast() {
        let cfg = TlsConfig::Pem {
            path: "/this/path/does/not/exist.pem".into(),
        };
        let err = Builder::with_config(cfg).build().unwrap_err();
        match err {
            Error::FileReadFailed { path, .. } => assert!(path.contains("does/not/exist")),
            other => panic!("expected FileReadFailed, got {other:?}"),
        }
    }
}
