//! Custom-header strategy — for APIs that use non-standard
//! credential headers (`x-api-key`, `x-goog-api-key`,
//! `x-auth-token`, etc.) instead of `Authorization`.

use async_trait::async_trait;
use http::header::{HeaderName, HeaderValue};
use secrecy::{ExposeSecret, SecretString};

use crate::api::auth_strategy::AuthStrategy;
use crate::api::error::Error;

/// Attaches `<name>: <value>` to every outbound request.
pub(crate) struct HeaderStrategy {
    name: HeaderName,
    value: HeaderValue,
}

impl std::fmt::Debug for HeaderStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeaderStrategy")
            .field("name", &self.name.as_str())
            .field("value", &"<redacted>")
            .finish()
    }
}

impl HeaderStrategy {
    /// Construct from a user-supplied header name + resolved
    /// credential value. The name is lowercased before parse
    /// so consumer configs don't need to worry about casing
    /// (HTTP headers are case-insensitive on the wire anyway).
    pub(crate) fn new(name: String, value: SecretString) -> Result<Self, Error> {
        let lower = name.to_lowercase();
        let header_name =
            HeaderName::from_lowercase(lower.as_bytes()).map_err(|e| Error::InvalidHeaderName {
                name: name.clone(),
                reason: e.to_string(),
            })?;
        let mut header_value = HeaderValue::from_str(value.expose_secret())
            .map_err(|e| Error::InvalidHeaderValue(e.to_string()))?;
        header_value.set_sensitive(true);
        Ok(Self {
            name: header_name,
            value: header_value,
        })
    }
}

#[async_trait]
impl AuthStrategy for HeaderStrategy {
    fn authorize(&self, req: &mut reqwest::Request) -> Result<(), Error> {
        req.headers_mut().insert(self.name.clone(), self.value.clone());
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

    /// @covers: HeaderStrategy::authorize
    #[test]
    fn test_authorize_attaches_custom_header() {
        let s = HeaderStrategy::new(
            "x-api-key".into(),
            SecretString::from("k-12345".to_string()),
        )
        .unwrap();
        let mut req = stub_request();
        s.authorize(&mut req).unwrap();
        assert_eq!(
            req.headers().get("x-api-key").unwrap().to_str().unwrap(),
            "k-12345"
        );
    }

    /// @covers: HeaderStrategy::new
    #[test]
    fn test_new_lowercases_header_name_case_insensitively() {
        // Config might specify `X-API-Key`; we should accept it
        // and canonicalize.
        let s = HeaderStrategy::new(
            "X-API-Key".into(),
            SecretString::from("v".to_string()),
        )
        .unwrap();
        let mut req = stub_request();
        s.authorize(&mut req).unwrap();
        // Header-name matching on the wire is case-insensitive;
        // the stored name is lowercased.
        assert!(req.headers().contains_key("x-api-key"));
    }

    /// @covers: HeaderStrategy::new
    #[test]
    fn test_new_rejects_invalid_header_name() {
        // Spaces aren't allowed in header names per RFC 7230.
        let err = HeaderStrategy::new(
            "bad name".into(),
            SecretString::from("v".to_string()),
        )
        .unwrap_err();
        match err {
            Error::InvalidHeaderName { name, .. } => assert_eq!(name, "bad name"),
            other => panic!("expected InvalidHeaderName, got {other:?}"),
        }
    }

    /// @covers: HeaderStrategy::new
    #[test]
    fn test_new_rejects_invalid_header_value() {
        let err = HeaderStrategy::new(
            "x-key".into(),
            SecretString::from("bad\nvalue".to_string()),
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidHeaderValue(_)));
    }

    /// @covers: HeaderStrategy::fmt
    #[test]
    fn test_fmt_debug_shows_name_and_redacts_value() {
        let s = HeaderStrategy::new(
            "x-key".into(),
            SecretString::from("s3cr3t".to_string()),
        )
        .unwrap();
        let dbg = format!("{s:?}");
        assert!(dbg.contains("HeaderStrategy"));
        assert!(!dbg.contains("s3cr3t"));
    }

    /// @covers: HeaderStrategy (Debug impl)
    #[test]
    fn test_debug_impl_shows_name_but_redacts_value() {
        let s = HeaderStrategy::new(
            "x-api-key".into(),
            SecretString::from("super_unique_secret".to_string()),
        )
        .unwrap();
        let s_dbg = format!("{s:?}");
        assert!(s_dbg.contains("x-api-key"));
        assert!(!s_dbg.contains("super_unique_secret"));
        assert!(s_dbg.contains("redacted"));
    }
}
