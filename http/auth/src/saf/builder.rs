//! Public builder entry point.
//!
//! Consumers construct an [`AuthConfig`] — usually via
//! [`AuthConfig::swe_default`] (the pass-through baseline) or
//! [`AuthConfig::from_config`] with their own TOML — then hand
//! it to the builder. Policy lives in config files, not in
//! chained method calls.

use crate::api::auth_config::AuthConfig;
use crate::api::error::Error;

/// Start configuring the auth middleware with the SWE baseline
/// loaded from the crate-shipped `config/default.toml`, which
/// is `kind = "none"` (pass-through).
pub fn builder() -> Result<Builder, Error> {
    let cfg = AuthConfig::swe_default()?;
    Ok(Builder { config: cfg })
}

/// Builder handle. Opaque — knobs live on the config.
#[derive(Debug)]
pub struct Builder {
    config: AuthConfig,
}

impl Builder {
    /// Construct from a caller-supplied config.
    pub fn with_config(config: AuthConfig) -> Self {
        Self { config }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &AuthConfig {
        &self.config
    }

    /// Finalize into the middleware layer. Scaffold phase:
    /// returns [`Error::NotImplemented`]. When the real impl
    /// lands, this will:
    ///   1. For `AuthConfig::None`: return a pass-through layer.
    ///   2. For others: resolve the named env var(s); if any
    ///      are missing, return [`Error::MissingEnvVar`]; else
    ///      build a layer that attaches the credential header
    ///      to every outbound request.
    pub fn build(self) -> Result<(), Error> {
        Err(Error::NotImplemented("builder"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: builder
    #[test]
    fn test_builder_loads_swe_default_which_is_none_pass_through() {
        let b = builder().expect("baseline must parse");
        assert!(matches!(b.config(), AuthConfig::None));
    }

    /// @covers: Builder::with_config
    #[test]
    fn test_with_config_holds_supplied_policy() {
        let cfg = AuthConfig::from_config(
            r#"
                kind = "bearer"
                token_env = "SOME_ENV"
            "#,
        )
        .unwrap();
        let b = Builder::with_config(cfg);
        assert!(matches!(b.config(), AuthConfig::Bearer { .. }));
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_returns_not_implemented_during_scaffold_phase() {
        let b = builder().expect("baseline parses");
        let err = b.build().unwrap_err();
        assert!(matches!(err, Error::NotImplemented(_)));
    }
}
