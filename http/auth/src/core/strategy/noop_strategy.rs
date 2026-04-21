//! Pass-through strategy for `AuthConfig::None`.

use crate::api::auth_strategy::AuthStrategy;
use crate::api::error::Error;

/// Attaches no credential. Returned by the factory when
/// `AuthConfig::None` is configured (the baseline).
#[derive(Debug, Default)]
pub(crate) struct NoopStrategy;

impl AuthStrategy for NoopStrategy {
    fn authorize(&self, _req: &mut reqwest::Request) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Method, Url};

    /// @covers: NoopStrategy
    #[test]
    fn test_noop_does_not_modify_request_or_add_headers() {
        let s = NoopStrategy;
        let mut req = reqwest::Request::new(
            Method::GET,
            Url::parse("http://example.test/").unwrap(),
        );
        let before = req.headers().len();
        s.authorize(&mut req).expect("noop never fails");
        assert_eq!(req.headers().len(), before);
        assert!(req.headers().get("authorization").is_none());
    }
}
