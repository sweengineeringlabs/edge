//! HTTP egress trait — makes outbound HTTP requests.

use crate::api::egress_error::EgressError;

/// Makes outbound HTTP requests to external services.
pub trait HttpClient: Send + Sync {
    /// A description of this HTTP client for diagnostics.
    fn describe(&self) -> &'static str;

    /// Verify the client can reach the configured base URL.
    fn health_check(&self) -> Result<(), EgressError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubClient;
    impl HttpClient for StubClient {
        fn describe(&self) -> &'static str { "stub" }
        fn health_check(&self) -> Result<(), EgressError> { Ok(()) }
    }

    #[test]
    fn test_http_client_describe_returns_str() {
        assert_eq!(StubClient.describe(), "stub");
    }

    #[test]
    fn test_http_client_health_check_ok_succeeds() {
        assert!(StubClient.health_check().is_ok());
    }
}
