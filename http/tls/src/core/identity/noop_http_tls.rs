//! Pass-through — no client identity attached.

use crate::api::error::Error;
use crate::api::http_tls::HttpTls;

#[derive(Debug, Default)]
pub(crate) struct NoopHttpTls;

impl HttpTls for NoopHttpTls {
    fn describe(&self) -> &'static str {
        "noop"
    }

    fn identity(&self) -> Result<Option<reqwest::Identity>, Error> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: NoopHttpTls
    #[test]
    fn test_noop_returns_no_identity() {
        let p = NoopHttpTls;
        assert!(p.identity().unwrap().is_none());
        assert_eq!(p.describe(), "noop");
    }
}
