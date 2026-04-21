//! Pluggable HTTP auth strategy contract.
//!
//! One impl per scheme (Bearer, Basic, custom Header, Noop).
//! The factory in `core::strategy::strategy_factory` turns an
//! `AuthConfig` + `CredentialResolver` into a concrete
//! `Box<dyn AuthStrategy>` at `Builder::build()` time.
//!
//! `pub(crate)` on purpose — consumers never implement this
//! trait. Plug-in extension points are scoped to new variants
//! in [`AuthConfig`](crate::api::auth_config::AuthConfig), not
//! to arbitrary external impls.

use crate::api::error::Error;

/// Attaches configured credentials to an outbound HTTP request.
///
/// Implementations hold any pre-computed state they need
/// (encoded header value, header name, etc.) so the hot path
/// on every request is a trivial insert.
pub(crate) trait AuthStrategy: Send + Sync + std::fmt::Debug {
    /// Apply the strategy to `req` in place. Called once per
    /// outbound request by the middleware layer.
    ///
    /// Returns an error only if the credential produces a
    /// malformed header value at attach time (should be
    /// impossible if construction validated the bytes, but
    /// defense-in-depth against config drift).
    fn authorize(&self, req: &mut reqwest::Request) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Method, Url};

    #[derive(Debug)]
    struct StubStrategy;
    impl AuthStrategy for StubStrategy {
        fn authorize(&self, req: &mut reqwest::Request) -> Result<(), Error> {
            req.headers_mut()
                .insert("x-stub", "applied".parse().unwrap());
            Ok(())
        }
    }

    /// @covers: AuthStrategy
    #[test]
    fn test_trait_is_dyn_compatible_and_mutates_request() {
        let s: Box<dyn AuthStrategy> = Box::new(StubStrategy);
        let mut req = reqwest::Request::new(
            Method::GET,
            Url::parse("http://example.test/").unwrap(),
        );
        s.authorize(&mut req).unwrap();
        assert_eq!(req.headers().get("x-stub").unwrap(), "applied");
    }
}
