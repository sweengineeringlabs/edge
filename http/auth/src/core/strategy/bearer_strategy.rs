//! Bearer-token strategy (RFC 6750).
//!
//! Attaches `Authorization: Bearer <token>` to every outbound
//! request. Pre-computes the header value at construction so
//! the hot path on each request is a header insert.

use async_trait::async_trait;
use http::header::{HeaderValue, AUTHORIZATION};
use secrecy::{ExposeSecret, SecretString};

use crate::api::auth_strategy::AuthStrategy;
use crate::api::error::Error;

/// `Authorization: Bearer <token>` strategy.
pub(crate) struct BearerStrategy {
    /// Pre-computed `Bearer <token>` header value, marked
    /// sensitive so `Debug` doesn't leak it.
    header_value: HeaderValue,
}

impl std::fmt::Debug for BearerStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Don't print the header value — even the sensitive flag
        // wouldn't fully hide the raw bytes on some
        // alternate-format pathways. Keep the type name only.
        f.debug_struct("BearerStrategy")
            .field("header_value", &"<redacted>")
            .finish()
    }
}

impl BearerStrategy {
    /// Construct from a resolved token. Returns
    /// [`Error::InvalidHeaderValue`] if the token contains
    /// characters forbidden in an HTTP header value (CR, LF,
    /// NUL, non-visible ASCII).
    pub(crate) fn new(token: SecretString) -> Result<Self, Error> {
        let raw = format!("Bearer {}", token.expose_secret());
        let mut hv = HeaderValue::from_str(&raw)
            .map_err(|e| Error::InvalidHeaderValue(e.to_string()))?;
        hv.set_sensitive(true);
        Ok(Self { header_value: hv })
    }
}

#[async_trait]
impl AuthStrategy for BearerStrategy {
    fn authorize(&self, req: &mut reqwest::Request) -> Result<(), Error> {
        req.headers_mut()
            .insert(AUTHORIZATION, self.header_value.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Method, Url};

    fn stub_request() -> reqwest::Request {
        reqwest::Request::new(
            Method::GET,
            Url::parse("http://example.test/").unwrap(),
        )
    }

    /// @covers: BearerStrategy::authorize
    #[test]
    fn test_authorize_attaches_bearer_authorization_header() {
        let s = BearerStrategy::new(SecretString::from("sk-abc123".to_string())).unwrap();
        let mut req = stub_request();
        s.authorize(&mut req).unwrap();
        let h = req.headers().get("authorization").unwrap();
        assert_eq!(h.to_str().unwrap(), "Bearer sk-abc123");
    }

    /// @covers: BearerStrategy::authorize
    #[test]
    fn test_authorize_overwrites_existing_authorization_header() {
        let s = BearerStrategy::new(SecretString::from("new-token".to_string())).unwrap();
        let mut req = stub_request();
        req.headers_mut()
            .insert("authorization", "Bearer old-token".parse().unwrap());
        s.authorize(&mut req).unwrap();
        assert_eq!(
            req.headers().get("authorization").unwrap().to_str().unwrap(),
            "Bearer new-token"
        );
    }

    /// @covers: BearerStrategy::new
    #[test]
    fn test_new_rejects_token_with_newline() {
        // \n is forbidden in header values per RFC 7230.
        let err = BearerStrategy::new(SecretString::from("bad\ntoken".to_string())).unwrap_err();
        assert!(matches!(err, Error::InvalidHeaderValue(_)));
    }

    /// @covers: BearerStrategy (Debug impl)
    #[test]
    fn test_debug_impl_does_not_leak_token() {
        let s = BearerStrategy::new(SecretString::from("super-secret".to_string())).unwrap();
        let s_dbg = format!("{s:?}");
        assert!(!s_dbg.contains("super-secret"));
        assert!(s_dbg.contains("redacted") || s_dbg.contains("BearerStrategy"));
    }
}
