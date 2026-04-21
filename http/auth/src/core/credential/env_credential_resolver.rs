//! Environment-variable-backed [`CredentialResolver`].

use secrecy::SecretString;

use crate::api::credential_resolver::CredentialResolver;
use crate::api::credential_source::CredentialSource;
use crate::api::error::Error;

/// [`CredentialResolver`] impl that reads values from process
/// env vars via [`std::env::var`].
///
/// Stateless — no configuration needed; construct with
/// `EnvCredentialResolver` directly or through
/// `EnvCredentialResolver::default()`.
#[derive(Debug, Default)]
pub(crate) struct EnvCredentialResolver;

impl CredentialResolver for EnvCredentialResolver {
    fn resolve(&self, source: &CredentialSource) -> Result<SecretString, Error> {
        match source {
            CredentialSource::EnvVar(name) => {
                std::env::var(name).map(SecretString::from).map_err(|_| {
                    Error::MissingEnvVar {
                        name: name.clone(),
                    }
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    // Env-var tests are serialized by using unique variable
    // names per test so concurrent test threads don't race.

    /// @covers: EnvCredentialResolver::resolve
    #[test]
    fn test_resolve_env_var_present_returns_value() {
        std::env::set_var("EDGE_TEST_TOKEN_PRESENT_01", "hello-bearer");
        let resolver = EnvCredentialResolver;
        let src = CredentialSource::EnvVar("EDGE_TEST_TOKEN_PRESENT_01".into());
        let secret = resolver.resolve(&src).expect("present");
        assert_eq!(secret.expose_secret(), "hello-bearer");
        std::env::remove_var("EDGE_TEST_TOKEN_PRESENT_01");
    }

    /// @covers: EnvCredentialResolver::resolve
    #[test]
    fn test_resolve_env_var_missing_returns_missing_env_var_error() {
        // Use a name we can be sure isn't set.
        std::env::remove_var("EDGE_TEST_TOKEN_ABSENT_01");
        let resolver = EnvCredentialResolver;
        let src = CredentialSource::EnvVar("EDGE_TEST_TOKEN_ABSENT_01".into());
        let err = resolver.resolve(&src).unwrap_err();
        match err {
            Error::MissingEnvVar { name } => {
                assert_eq!(name, "EDGE_TEST_TOKEN_ABSENT_01");
            }
            other => panic!("expected MissingEnvVar, got {other:?}"),
        }
    }

    /// @covers: EnvCredentialResolver::resolve
    #[test]
    fn test_resolve_empty_env_var_is_still_ok_scheme_validates_shape() {
        // Empty-string env var = still "set" at OS level. Whether
        // empty is acceptable is a scheme-level concern, not the
        // resolver's. This test pins that contract.
        std::env::set_var("EDGE_TEST_TOKEN_EMPTY_01", "");
        let resolver = EnvCredentialResolver;
        let src = CredentialSource::EnvVar("EDGE_TEST_TOKEN_EMPTY_01".into());
        let secret = resolver.resolve(&src).expect("present even if empty");
        assert_eq!(secret.expose_secret(), "");
        std::env::remove_var("EDGE_TEST_TOKEN_EMPTY_01");
    }
}
