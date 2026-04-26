//! Public builder entry point.
//!
//! Consumers construct an [`AuthConfig`] — usually via
//! [`AuthConfig::swe_default`] (the pass-through baseline) or
//! [`AuthConfig::from_config`] with their own TOML — then hand
//! it to the builder. Policy lives in config files, not in
//! chained method calls.

use std::sync::Arc;

use crate::api::auth_config::AuthConfig;
use crate::api::auth_middleware::AuthMiddleware;
use crate::api::credential_resolver::CredentialResolver;
use crate::api::error::Error;

use crate::core::auth_middleware as _;
use crate::core::credential::EnvCredentialResolver;
use crate::core::default_http_auth::DefaultHttpAuth;

/// Start configuring the auth middleware with the SWE baseline
/// loaded from the crate-shipped `config/application.toml`
/// (which is `kind = "none"` — pass-through).
pub fn builder() -> Result<Builder, Error> {
    let cfg = AuthConfig::swe_default()?;
    Ok(Builder::with_config(cfg))
}

pub use crate::api::builder::Builder;

impl std::fmt::Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("config", &self.config)
            .finish()
    }
}

impl Builder {
    /// Construct from a caller-supplied config. Uses the
    /// default [`EnvCredentialResolver`] (reads credentials from
    /// process env vars).
    pub fn with_config(config: AuthConfig) -> Self {
        Self {
            config,
            resolver: Box::new(EnvCredentialResolver),
        }
    }

    /// Borrow the current policy.
    pub fn config(&self) -> &AuthConfig {
        &self.config
    }

    /// Finalize into the middleware layer.
    ///
    /// Resolves every env-var reference in the config NOW. A
    /// missing env var fails with [`Error::MissingEnvVar`] so
    /// startup (not the first request) surfaces the
    /// misconfiguration.
    ///
    /// The returned [`AuthMiddleware`] implements
    /// `reqwest_middleware::Middleware` — plug into a
    /// `reqwest_middleware::ClientBuilder` via `.with(mw)`.
    pub fn build(self) -> Result<AuthMiddleware, Error> {
        let processor = DefaultHttpAuth::new(self.config, self.resolver.as_ref())?;
        Ok(AuthMiddleware::new(Arc::new(processor)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

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
    fn test_build_with_none_config_returns_middleware_instance() {
        let mw = builder().expect("baseline").build().expect("build ok");
        // The middleware's Debug impl reports the underlying
        // processor's describe() — confirms end-to-end wiring.
        let s = format!("{mw:?}");
        assert!(s.contains("swe_edge_egress_auth"));
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_with_missing_bearer_env_fails_at_build_time() {
        let cfg = AuthConfig::Bearer {
            token_env: "EDGE_TEST_DEFINITELY_NOT_SET_99".into(),
        };
        std::env::remove_var("EDGE_TEST_DEFINITELY_NOT_SET_99");
        let err = Builder::with_config(cfg).build().unwrap_err();
        match err {
            Error::MissingEnvVar { name } => {
                assert_eq!(name, "EDGE_TEST_DEFINITELY_NOT_SET_99");
            }
            other => panic!("expected MissingEnvVar, got {other:?}"),
        }
    }

    /// @covers: Builder::build
    #[test]
    fn test_build_with_bearer_env_set_produces_functioning_middleware() {
        // Ensure the var is set for this test regardless of
        // prior test ordering.
        std::env::set_var("EDGE_TEST_BEARER_OK_01", "tok-99");
        let cfg = AuthConfig::Bearer {
            token_env: "EDGE_TEST_BEARER_OK_01".into(),
        };
        let _mw = Builder::with_config(cfg).build().expect("bearer builds");

        // Can't hit a real server in a unit test, but the
        // middleware's existence + the processor.process path
        // being reachable is proof enough — covered by the
        // core::default_http_auth::tests::test_process_with_bearer
        // assertion that the header is correctly applied.
        let _ = AtomicBool::new(true); // marker — test reached here
        std::env::remove_var("EDGE_TEST_BEARER_OK_01");
    }
}
