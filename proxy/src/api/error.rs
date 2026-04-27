//! Error types for proxy dispatch concerns.

use thiserror::Error;

pub use edge_domain::HandlerError;

/// Errors raised by [`Job::run`](crate::Job::run).
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

    /// Domain-specific failure that does not fit the above categories.
    #[error("job error: {0}")]
    Other(String),
}

/// Errors raised by [`Router::route`](crate::Router::route).
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

/// Errors raised by [`LifecycleMonitor::shutdown`](crate::LifecycleMonitor::shutdown).
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
