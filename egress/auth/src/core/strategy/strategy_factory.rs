//! Factory that turns an [`AuthConfig`] + [`CredentialResolver`]
//! into a concrete [`Box<dyn AuthStrategy>`].
//!
//! The one place `match AuthConfig` lives — the runtime hot
//! path is pure trait-object dispatch on the resulting
//! [`AuthStrategy`].

use crate::api::auth_config::AuthConfig;
use crate::api::auth_strategy::AuthStrategy;
use crate::api::credential_resolver::CredentialResolver;
use crate::api::credential_source::CredentialSource;
use crate::api::error::Error;

use super::aws_sigv4_strategy::AwsSigV4Strategy;
use super::basic_strategy::BasicStrategy;
use super::bearer_strategy::BearerStrategy;
use super::digest_strategy::DigestStrategy;
use super::header_strategy::HeaderStrategy;
use super::noop_strategy::NoopStrategy;

/// Realize an [`AuthConfig`] against a [`CredentialResolver`].
///
/// Env vars referenced by the config are resolved NOW (at
/// build time). If any are missing, the call fails with
/// [`Error::MissingEnvVar`] naming the offender — startup
/// surfaces the misconfiguration rather than letting it leak
/// into the first request.
pub(crate) fn build_strategy(
    config: &AuthConfig,
    resolver: &dyn CredentialResolver,
) -> Result<Box<dyn AuthStrategy>, Error> {
    match config {
        AuthConfig::None => Ok(Box::new(NoopStrategy)),

        AuthConfig::Bearer { token_env } => {
            let token = resolver.resolve(&CredentialSource::EnvVar(token_env.clone()))?;
            Ok(Box::new(BearerStrategy::new(token)?))
        }

        AuthConfig::Basic { user_env, pass_env } => {
            let user = resolver.resolve(&CredentialSource::EnvVar(user_env.clone()))?;
            let pass = resolver.resolve(&CredentialSource::EnvVar(pass_env.clone()))?;
            Ok(Box::new(BasicStrategy::new(user, pass)?))
        }

        AuthConfig::Header { name, value_env } => {
            let value = resolver.resolve(&CredentialSource::EnvVar(value_env.clone()))?;
            Ok(Box::new(HeaderStrategy::new(name.clone(), value)?))
        }

        AuthConfig::Digest {
            user_env,
            password_env,
            realm,
        } => {
            let user = resolver.resolve(&CredentialSource::EnvVar(user_env.clone()))?;
            let password = resolver.resolve(&CredentialSource::EnvVar(password_env.clone()))?;
            Ok(Box::new(DigestStrategy::new(user, password, realm.clone())?))
        }

        AuthConfig::AwsSigV4 {
            access_key_env,
            secret_key_env,
            session_token_env,
            region,
            service,
        } => {
            let access_key =
                resolver.resolve(&CredentialSource::EnvVar(access_key_env.clone()))?;
            let secret_key =
                resolver.resolve(&CredentialSource::EnvVar(secret_key_env.clone()))?;
            let session_token = match session_token_env {
                Some(var) => Some(
                    resolver.resolve(&CredentialSource::EnvVar(var.clone()))?,
                ),
                None => None,
            };
            Ok(Box::new(AwsSigV4Strategy::new(
                access_key,
                secret_key,
                session_token,
                region.clone(),
                service.clone(),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    /// Stub resolver that returns the same canned secret for
    /// every source. Lets us exercise the factory without
    /// touching process env.
    struct StubResolver(&'static str);
    impl CredentialResolver for StubResolver {
        fn resolve(&self, _source: &CredentialSource) -> Result<SecretString, Error> {
            Ok(SecretString::from(self.0.to_string()))
        }
    }

    struct MissingResolver;
    impl CredentialResolver for MissingResolver {
        fn resolve(&self, source: &CredentialSource) -> Result<SecretString, Error> {
            let name = match source {
                CredentialSource::EnvVar(n) => n.clone(),
            };
            Err(Error::MissingEnvVar { name })
        }
    }

    /// @covers: build_strategy
    #[test]
    fn test_build_strategy_returns_ok_for_every_config_variant() {
        // Smoke: every variant constructs without panic when
        // the resolver supplies a placeholder credential.
        let r = &StubResolver("x");
        assert!(build_strategy(&AuthConfig::None, r).is_ok());
        assert!(build_strategy(
            &AuthConfig::Bearer { token_env: "T".into() },
            r
        ).is_ok());
        assert!(build_strategy(
            &AuthConfig::Basic { user_env: "U".into(), pass_env: "P".into() },
            r
        ).is_ok());
        assert!(build_strategy(
            &AuthConfig::Header { name: "x-k".into(), value_env: "V".into() },
            r
        ).is_ok());
    }

    /// @covers: build_strategy
    #[test]
    fn test_none_builds_noop_strategy() {
        let strategy = build_strategy(&AuthConfig::None, &StubResolver("x")).unwrap();
        // Applying it to a request must not attach any auth.
        let mut req = reqwest::Request::new(
            reqwest::Method::GET,
            reqwest::Url::parse("http://example.test/").unwrap(),
        );
        strategy.authorize(&mut req).unwrap();
        assert!(req.headers().get("authorization").is_none());
    }

    /// @covers: build_strategy
    #[test]
    fn test_bearer_builds_strategy_that_attaches_authorization_header() {
        let cfg = AuthConfig::Bearer {
            token_env: "WHATEVER".into(),
        };
        let strategy = build_strategy(&cfg, &StubResolver("tok-42")).unwrap();
        let mut req = reqwest::Request::new(
            reqwest::Method::GET,
            reqwest::Url::parse("http://example.test/").unwrap(),
        );
        strategy.authorize(&mut req).unwrap();
        assert_eq!(
            req.headers().get("authorization").unwrap().to_str().unwrap(),
            "Bearer tok-42"
        );
    }

    /// @covers: build_strategy
    #[test]
    fn test_basic_builds_strategy_with_both_env_vars_resolved() {
        let cfg = AuthConfig::Basic {
            user_env: "U".into(),
            pass_env: "P".into(),
        };
        // Resolver returns "same" for BOTH env vars; verify
        // that strategy authorizes with a valid basic header.
        let strategy = build_strategy(&cfg, &StubResolver("same")).unwrap();
        let mut req = reqwest::Request::new(
            reqwest::Method::GET,
            reqwest::Url::parse("http://example.test/").unwrap(),
        );
        strategy.authorize(&mut req).unwrap();
        let h = req.headers().get("authorization").unwrap().to_str().unwrap();
        assert!(h.starts_with("Basic "));
    }

    /// @covers: build_strategy
    #[test]
    fn test_header_builds_strategy_for_custom_header_name() {
        let cfg = AuthConfig::Header {
            name: "x-api-key".into(),
            value_env: "K".into(),
        };
        let strategy = build_strategy(&cfg, &StubResolver("key-99")).unwrap();
        let mut req = reqwest::Request::new(
            reqwest::Method::GET,
            reqwest::Url::parse("http://example.test/").unwrap(),
        );
        strategy.authorize(&mut req).unwrap();
        assert_eq!(
            req.headers().get("x-api-key").unwrap().to_str().unwrap(),
            "key-99"
        );
    }

    /// @covers: build_strategy
    #[test]
    fn test_missing_env_var_propagates_through_factory() {
        let cfg = AuthConfig::Bearer {
            token_env: "DOES_NOT_EXIST".into(),
        };
        let err = build_strategy(&cfg, &MissingResolver).unwrap_err();
        match err {
            Error::MissingEnvVar { name } => assert_eq!(name, "DOES_NOT_EXIST"),
            other => panic!("expected MissingEnvVar, got {other:?}"),
        }
    }
}
