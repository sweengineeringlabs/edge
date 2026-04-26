//! Trait for resolving a [`CredentialSource`] into a concrete
//! secret value.
//!
//! The real impl lives in [`core::credential::env_credential_resolver`]
//! and reads from process env vars. Test impls stub the resolver
//! with canned values so scheme logic can be exercised without
//! setting process env state.

use secrecy::SecretString;

use crate::api::credential_source::CredentialSource;
use crate::api::error::Error;

/// Resolves an abstract [`CredentialSource`] to its concrete
/// [`SecretString`] at middleware-build time.
///
/// Sync on purpose: env resolution is trivially sync, and
/// resolution happens exactly once per `Builder::build()` call
/// (not per request). If a future resolver needs async I/O
/// (vault, secret manager), the trait signature bumps and the
/// change propagates through every impl — acceptable one-time
/// cost.
pub(crate) trait CredentialResolver: Send + Sync {
    /// Resolve `source` to a concrete credential.
    ///
    /// Returns [`Error::MissingEnvVar`] when the source refers
    /// to a name that isn't set. Other error variants (bad
    /// format, empty value) are not this trait's concern —
    /// resolvers return whatever was at the named location; the
    /// scheme validates shape.
    fn resolve(&self, source: &CredentialSource) -> Result<SecretString, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    struct StubResolver(String);
    impl CredentialResolver for StubResolver {
        fn resolve(&self, _source: &CredentialSource) -> Result<SecretString, Error> {
            Ok(SecretString::from(self.0.clone()))
        }
    }

    /// @covers: CredentialResolver
    #[test]
    fn test_trait_is_dyn_compatible() {
        let r: Box<dyn CredentialResolver> = Box::new(StubResolver("stub-token".into()));
        let src = CredentialSource::EnvVar("WHATEVER".into());
        let secret = r.resolve(&src).unwrap();
        assert_eq!(secret.expose_secret(), "stub-token");
    }
}
