//! Error types — one per concern of the 5-Concern Controller pattern.
//!
//! Kept as separate enums so callers can pattern-match narrowly on the
//! concern that failed, and so boundary types between concerns stay
//! explicit (no single "god error").

use thiserror::Error;

/// Errors raised by `Job::run`.
#[derive(Debug, Error)]
pub enum JobError {
    /// The requested handler was not registered or not available.
    #[error("handler not available: {0}")]
    HandlerUnavailable(String),

    /// Routing failed before a handler could be chosen.
    #[error("routing failed: {0}")]
    Routing(#[from] RoutingError),

    /// The chosen handler failed during execution.
    #[error("handler failed: {0}")]
    Handler(#[from] HandlerError),

    /// The job was cancelled by a lifecycle event (e.g. shutdown).
    #[error("job cancelled")]
    Cancelled,

    /// Anything the domain wants to surface as a job-level failure that
    /// doesn't fit the above categories.
    #[error("job error: {0}")]
    Other(String),
}

/// Errors raised by `Router::route`.
#[derive(Debug, Error)]
pub enum RoutingError {
    /// Input was empty or otherwise unusable.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Input was valid but did not match any registered intent.
    #[error("no intent matched")]
    NoMatch,

    /// Domain-specific failure raised while classifying.
    #[error("routing error: {0}")]
    Other(String),
}

/// Errors raised by `Handler::execute`.
#[derive(Debug, Error)]
pub enum HandlerError {
    /// The handler was asked to do something it doesn't support.
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

    /// Anything the domain wants to surface that does not fit the above.
    #[error("handler error: {0}")]
    Other(String),
}

/// Errors raised by `LifecycleMonitor::shutdown` (and related lifecycle ops).
#[derive(Debug, Error)]
pub enum LifecycleError {
    /// Shutdown was called twice or on an already-stopped instance.
    #[error("already shut down")]
    AlreadyShutDown,

    /// A background task failed to stop cleanly.
    #[error("background task did not drain: {0}")]
    DrainFailed(String),

    /// Domain-specific lifecycle failure.
    #[error("lifecycle error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_error_wraps_routing_error() {
        let r: JobError = RoutingError::NoMatch.into();
        assert!(matches!(r, JobError::Routing(RoutingError::NoMatch)));
    }

    #[test]
    fn test_job_error_wraps_handler_error() {
        let h: JobError = HandlerError::Unhealthy.into();
        assert!(matches!(h, JobError::Handler(HandlerError::Unhealthy)));
    }

    #[test]
    fn test_lifecycle_error_display() {
        let err = LifecycleError::DrainFailed("worker A".to_string());
        assert!(err.to_string().contains("worker A"));
    }
}
