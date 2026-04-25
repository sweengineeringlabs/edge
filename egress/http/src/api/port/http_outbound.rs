//! HTTP outbound trait — makes outbound HTTP requests.

use futures::future::BoxFuture;
use thiserror::Error;

use crate::api::value_object::{HttpRequest, HttpResponse};

/// Error type for HTTP outbound operations.
#[derive(Debug, Error)]
pub enum HttpOutboundError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Result type for HTTP outbound operations.
pub type HttpOutboundResult<T> = Result<T, HttpOutboundError>;

/// Makes outbound HTTP requests to external services.
pub trait HttpOutbound: Send + Sync {
    fn send(&self, request: HttpRequest) -> BoxFuture<'_, HttpOutboundResult<HttpResponse>>;
    fn health_check(&self) -> BoxFuture<'_, HttpOutboundResult<()>>;

    fn get(&self, url: &str) -> BoxFuture<'_, HttpOutboundResult<HttpResponse>> {
        let req = HttpRequest::get(url.to_string());
        self.send(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_outbound_is_object_safe() {
        fn _assert_object_safe(_: &dyn HttpOutbound) {}
    }
}
