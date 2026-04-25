//! Default impl of [`HttpAuth`](crate::api::http_auth::HttpAuth).
//!
//! Holds a pre-resolved [`AuthStrategy`] (constructed once at
//! `build()` time from the config + resolver) and delegates
//! `process()` to it on every request.

use async_trait::async_trait;

use crate::api::auth_config::AuthConfig;
use crate::api::auth_strategy::AuthStrategy;
use crate::api::credential_resolver::CredentialResolver;
use crate::api::error::Error;
use crate::api::http_auth::HttpAuth;

use crate::core::strategy::build_strategy;

/// Default HTTP auth processor. Holds the resolved strategy;
/// per-request work is just `strategy.authorize(req)`.
pub(crate) struct DefaultHttpAuth {
    /// Kept for `describe()` / diagnostics — the config as
    /// declared, before resolution.
    #[allow(dead_code)]
    config: AuthConfig,
    /// Pre-resolved strategy realizing the config.
    strategy: Box<dyn AuthStrategy>,
}

impl std::fmt::Debug for DefaultHttpAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultHttpAuth")
            .field("config", &self.config)
            .field("strategy", &self.strategy)
            .finish()
    }
}

impl DefaultHttpAuth {
    /// Build from a config + credential resolver. Resolves all
    /// env-var references NOW — startup fails with
    /// [`Error::MissingEnvVar`] if any referenced variable is
    /// unset.
    pub(crate) fn build(
        config: AuthConfig,
        resolver: &dyn CredentialResolver,
    ) -> Result<Self, Error> {
        let strategy = build_strategy(&config, resolver)?;
        Ok(Self { config, strategy })
    }
}

#[async_trait]
impl HttpAuth for DefaultHttpAuth {
    fn describe(&self) -> &'static str {
        "swe_http_auth"
    }

    async fn process(&self, req: &mut reqwest::Request) -> Result<(), Error> {
        // Two-phase: first, any strategy-specific setup (Digest
        // fetches nonce here), then attach header.
        let host = req.url().host_str();
        self.strategy.prepare(host).await?;
        self.strategy.authorize(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::credential_source::CredentialSource;
    use secrecy::SecretString;

    struct StubResolver(&'static str);
    impl CredentialResolver for StubResolver {
        fn resolve(&self, _source: &CredentialSource) -> Result<SecretString, Error> {
            Ok(SecretString::from(self.0.to_string()))
        }
    }

    fn stub_request() -> reqwest::Request {
        reqwest::Request::new(
            reqwest::Method::GET,
            reqwest::Url::parse("http://example.test/").unwrap(),
        )
    }

    /// @covers: DefaultHttpAuth::describe
    #[test]
    fn test_describe_returns_crate_name() {
        let cfg = AuthConfig::swe_default().expect("baseline parses");
        let d = DefaultHttpAuth::build(cfg, &StubResolver("x")).expect("build ok");
        assert_eq!(d.describe(), "swe_http_auth");
    }

    /// @covers: DefaultHttpAuth::process
    #[tokio::test]
    async fn test_process_with_none_config_adds_no_header() {
        let d = DefaultHttpAuth::build(AuthConfig::None, &StubResolver("x")).unwrap();
        let mut req = stub_request();
        d.process(&mut req).await.unwrap();
        assert!(req.headers().get("authorization").is_none());
    }

    /// @covers: DefaultHttpAuth::process
    #[tokio::test]
    async fn test_process_with_bearer_config_attaches_authorization() {
        let cfg = AuthConfig::Bearer {
            token_env: "X".into(),
        };
        let d = DefaultHttpAuth::build(cfg, &StubResolver("tok-7")).unwrap();
        let mut req = stub_request();
        d.process(&mut req).await.unwrap();
        assert_eq!(
            req.headers().get("authorization").unwrap().to_str().unwrap(),
            "Bearer tok-7"
        );
    }

    /// @covers: DefaultHttpAuth::build
    #[test]
    fn test_build_fails_fast_on_missing_env_var() {
        struct MissingResolver;
        impl CredentialResolver for MissingResolver {
            fn resolve(&self, source: &CredentialSource) -> Result<SecretString, Error> {
                match source {
                    CredentialSource::EnvVar(n) => Err(Error::MissingEnvVar { name: n.clone() }),
                }
            }
        }
        let cfg = AuthConfig::Bearer {
            token_env: "NOT_SET".into(),
        };
        let err = DefaultHttpAuth::build(cfg, &MissingResolver).unwrap_err();
        match err {
            Error::MissingEnvVar { name } => assert_eq!(name, "NOT_SET"),
            other => panic!("expected MissingEnvVar, got {other:?}"),
        }
    }
}
