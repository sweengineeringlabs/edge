//! Basic-auth strategy (RFC 7617).
//!
//! Attaches `Authorization: Basic base64(user:pass)`. Encoding
//! happens once at construction — per-request cost is just a
//! header clone.
//!
//! Note on charset: RFC 7617 §2.1 says the user-id and password
//! MUST be `UTF-8` when encoded, and the server decides whether
//! to accept other encodings. We take `SecretString` which is
//! already `String` (UTF-8) so this is natively correct — we
//! don't need a separate `charset=UTF-8` header parameter.

use base64::Engine;
use async_trait::async_trait;
use http::header::{HeaderValue, AUTHORIZATION};
use secrecy::{ExposeSecret, SecretString};

use crate::api::auth_strategy::AuthStrategy;
use crate::api::error::Error;

/// `Authorization: Basic base64(user:pass)` strategy.
pub(crate) struct BasicStrategy {
    header_value: HeaderValue,
}

impl std::fmt::Debug for BasicStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicStrategy")
            .field("header_value", &"<redacted>")
            .finish()
    }
}

impl BasicStrategy {
    /// Construct from resolved user + password. Returns
    /// [`Error::InvalidHeaderValue`] if the resulting header
    /// value has forbidden bytes (the base64 output itself is
    /// always valid, but this is defense-in-depth).
    pub(crate) fn new(user: SecretString, pass: SecretString) -> Result<Self, Error> {
        // Combine as `user:pass` BEFORE base64 per RFC 7617 §2.
        let combined = format!("{}:{}", user.expose_secret(), pass.expose_secret());
        let encoded = base64::engine::general_purpose::STANDARD.encode(combined);
        let raw = format!("Basic {encoded}");
        let mut hv = HeaderValue::from_str(&raw)
            .map_err(|e| Error::InvalidHeaderValue(e.to_string()))?;
        hv.set_sensitive(true);
        Ok(Self { header_value: hv })
    }
}

#[async_trait]
impl AuthStrategy for BasicStrategy {
    fn authorize(&self, req: &mut reqwest::Request) -> Result<(), Error> {
        req.headers_mut()
            .insert(AUTHORIZATION, self.header_value.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use reqwest::{Method, Url};

    fn stub_request() -> reqwest::Request {
        reqwest::Request::new(
            Method::GET,
            Url::parse("http://example.test/").unwrap(),
        )
    }

    /// @covers: BasicStrategy::authorize
    #[test]
    fn test_authorize_attaches_basic_authorization_with_base64_user_pass() {
        let s = BasicStrategy::new(
            SecretString::from("alice".to_string()),
            SecretString::from("s3cr3t".to_string()),
        )
        .unwrap();
        let mut req = stub_request();
        s.authorize(&mut req).unwrap();

        let header = req.headers().get("authorization").unwrap().to_str().unwrap();
        let expected_payload = base64::engine::general_purpose::STANDARD.encode("alice:s3cr3t");
        assert_eq!(header, format!("Basic {expected_payload}"));
    }

    /// @covers: BasicStrategy::authorize
    #[test]
    fn test_authorize_handles_utf8_password() {
        // RFC 7617 §2.1 mandates UTF-8 — test a non-ASCII password
        // to lock in that we don't accidentally mangle it.
        let s = BasicStrategy::new(
            SecretString::from("bob".to_string()),
            SecretString::from("pässwörd".to_string()),
        )
        .unwrap();
        let mut req = stub_request();
        s.authorize(&mut req).unwrap();

        let header = req.headers().get("authorization").unwrap().to_str().unwrap();
        let expected_payload = base64::engine::general_purpose::STANDARD.encode("bob:pässwörd");
        assert_eq!(header, format!("Basic {expected_payload}"));
    }

    /// @covers: BasicStrategy (Debug impl)
    #[test]
    fn test_debug_impl_does_not_leak_credentials() {
        let s = BasicStrategy::new(
            SecretString::from("alice".to_string()),
            SecretString::from("unique_password_xyz".to_string()),
        )
        .unwrap();
        let s_dbg = format!("{s:?}");
        assert!(!s_dbg.contains("alice"));
        assert!(!s_dbg.contains("unique_password_xyz"));
        assert!(s_dbg.contains("redacted") || s_dbg.contains("BasicStrategy"));
    }

    /// @covers: BasicStrategy::fmt
    #[test]
    fn test_fmt_debug_redacts_header_value() {
        let s = BasicStrategy::new(
            SecretString::from("user".to_string()),
            SecretString::from("pass".to_string()),
        )
        .unwrap();
        let dbg = format!("{s:?}");
        assert!(dbg.contains("BasicStrategy"));
        assert!(!dbg.contains("pass"));
    }

    /// @covers: BasicStrategy::new
    #[test]
    fn test_new_accepts_empty_password() {
        // Empty password is technically valid per RFC 7617 (the
        // combined string would be `user:`). Scheme-level
        // validation isn't this layer's concern.
        let s = BasicStrategy::new(
            SecretString::from("user".to_string()),
            SecretString::from(String::new()),
        )
        .expect("empty password is acceptable at this layer");
        let mut req = stub_request();
        s.authorize(&mut req).unwrap();
        assert!(req.headers().contains_key("authorization"));
    }
}
