//! HTTP inbound trait — receives HTTP requests from upstream callers.

use crate::api::ingress_error::IngressError;

/// Receives and deserialises inbound HTTP requests.
pub trait HttpInbound: Send + Sync {
    /// A description of this HTTP inbound adapter for diagnostics.
    fn describe(&self) -> &'static str;

    /// Verify the adapter is reachable and accepting requests.
    fn health_check(&self) -> Result<(), IngressError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubHttp;
    impl HttpInbound for StubHttp {
        fn describe(&self) -> &'static str { "stub" }
        fn health_check(&self) -> Result<(), IngressError> { Ok(()) }
    }

    #[test]
    fn test_http_inbound_describe_returns_str() {
        assert_eq!(StubHttp.describe(), "stub");
    }

    #[test]
    fn test_http_inbound_health_check_ok_succeeds() {
        assert!(StubHttp.health_check().is_ok());
    }
}
