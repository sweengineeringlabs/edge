//! Error types for domain execution units.

use thiserror::Error;

/// Errors raised by [`Handler::execute`](crate::Handler::execute).
#[derive(Debug, Error)]
pub enum HandlerError {
    /// The handler was asked to do something it does not support.
    #[error("unsupported operation: {0}")]
    Unsupported(String),

    /// Handler input was malformed.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Handler ran to completion but the execution did not succeed.
    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    /// The handler is currently unhealthy and refused the request.
    #[error("handler unhealthy")]
    Unhealthy,

    /// Caller asked for an operation the handler cannot perform in its
    /// current state — distinct from `InvalidRequest` (request shape is
    /// fine; system state is not). Maps to gRPC `FAILED_PRECONDITION`.
    #[error("failed precondition: {0}")]
    FailedPrecondition(String),

    /// Domain-specific failure that does not fit the above categories.
    #[error("handler error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_error_display_contains_message() {
        let err = HandlerError::ExecutionFailed("timeout".to_string());
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_handler_error_unhealthy_display() {
        assert_eq!(HandlerError::Unhealthy.to_string(), "handler unhealthy");
    }
}
