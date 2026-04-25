//! HTTP inbound trait — handles incoming HTTP requests.

use futures::future::BoxFuture;

use crate::api::value_object::{HttpRequest, HttpResponse};

/// Result type for HTTP inbound operations.
pub type HttpInboundResult<T> = Result<T, HttpInboundError>;

/// Error type for HTTP inbound operations.
#[derive(Debug, thiserror::Error)]
pub enum HttpInboundError {
    #[error("internal: {0}")]
    Internal(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

/// Minimal health-check result for the HTTP domain.
#[derive(Debug, Clone)]
pub struct HttpHealthCheck {
    pub healthy: bool,
    pub message: Option<String>,
}

impl HttpHealthCheck {
    pub fn healthy() -> Self { Self { healthy: true, message: None } }
    pub fn unhealthy(msg: impl Into<String>) -> Self { Self { healthy: false, message: Some(msg.into()) } }
}

/// Receives and handles inbound HTTP requests.
pub trait HttpInbound: Send + Sync {
    fn handle(&self, request: HttpRequest) -> BoxFuture<'_, HttpInboundResult<HttpResponse>>;
    fn health_check(&self) -> BoxFuture<'_, HttpInboundResult<HttpHealthCheck>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_inbound_is_object_safe() {
        fn _assert_object_safe(_: &dyn HttpInbound) {}
    }

    #[test]
    fn test_http_inbound_error_internal_formats_correctly() {
        let err = HttpInboundError::Internal("oops".into());
        assert!(err.to_string().contains("oops"));
    }

    #[test]
    fn test_http_health_check_healthy_is_true() {
        let h = HttpHealthCheck::healthy();
        assert!(h.healthy);
        assert!(h.message.is_none());
    }

    #[test]
    fn test_http_health_check_unhealthy_sets_message() {
        let h = HttpHealthCheck::unhealthy("down");
        assert!(!h.healthy);
        assert_eq!(h.message.as_deref(), Some("down"));
    }
}
