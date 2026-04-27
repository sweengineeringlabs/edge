//! Inbound gateway error type.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result type for inbound gateway operations.
pub type IngressResult<T> = Result<T, IngressError>;

/// Standard error codes for inbound gateway operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngressErrorCode {
    /// Internal server error.
    Internal,
    /// Request input failed validation.
    InvalidInput,
    /// Resource not found.
    NotFound,
    /// Resource already exists.
    AlreadyExists,
    /// Caller lacks permission.
    PermissionDenied,
    /// Operation timed out.
    Timeout,
    /// Service unavailable.
    Unavailable,
    /// Configuration error.
    Configuration,
}

/// Comprehensive error type for inbound gateway operations.
#[derive(Debug, Error)]
pub enum IngressError {
    /// Connection to a backend failed.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Authentication credentials were rejected.
    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Resource not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Resource already exists (conflict).
    #[error("conflict: {0}")]
    Conflict(String),

    /// Input failed validation.
    #[error("validation error: {0}")]
    ValidationError(String),

    /// Rate limit was exceeded.
    #[error("rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Operation timed out.
    #[error("timeout: {0}")]
    Timeout(String),

    /// Operation is not supported.
    #[error("not supported: {0}")]
    NotSupported(String),

    /// Underlying I/O error.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialisation or deserialisation failure.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Backend returned an unexpected error.
    #[error("backend error: {0}")]
    BackendError(String),

    /// Internal server error.
    #[error("internal error: {0}")]
    InternalError(String),

    /// Attempted to create a resource that already exists.
    #[error("already exists: {0}")]
    AlreadyExists(String),

    /// Caller does not have the required permission.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Service is temporarily unavailable.
    #[error("unavailable: {0}")]
    Unavailable(String),

    /// Invalid or missing configuration.
    #[error("configuration error: {0}")]
    Configuration(String),
}

impl IngressError {
    /// Create an inbound error from a code and message.
    pub fn new(code: IngressErrorCode, message: impl Into<String>) -> Self {
        let msg = message.into();
        match code {
            IngressErrorCode::Internal => IngressError::InternalError(msg),
            IngressErrorCode::InvalidInput => IngressError::ValidationError(msg),
            IngressErrorCode::NotFound => IngressError::NotFound(msg),
            IngressErrorCode::AlreadyExists => IngressError::AlreadyExists(msg),
            IngressErrorCode::PermissionDenied => IngressError::PermissionDenied(msg),
            IngressErrorCode::Timeout => IngressError::Timeout(msg),
            IngressErrorCode::Unavailable => IngressError::Unavailable(msg),
            IngressErrorCode::Configuration => IngressError::Configuration(msg),
        }
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(IngressErrorCode::Internal, message)
    }

    /// Create a not-found error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(IngressErrorCode::NotFound, message)
    }

    /// Create an invalid-input error.
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(IngressErrorCode::InvalidInput, message)
    }

    /// Create an unavailable error.
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(IngressErrorCode::Unavailable, message)
    }

    /// Create an already-exists error.
    pub fn already_exists(message: impl Into<String>) -> Self {
        Self::new(IngressErrorCode::AlreadyExists, message)
    }

    /// Create a permission-denied error.
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(IngressErrorCode::PermissionDenied, message)
    }

    /// Create a timeout error.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(IngressErrorCode::Timeout, message)
    }

    /// Create a configuration error.
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::new(IngressErrorCode::Configuration, message)
    }

    /// Append a details string to the error message.
    pub fn with_details(self, details: impl Into<String>) -> Self {
        let d = details.into();
        match self {
            IngressError::ConnectionFailed(m) => IngressError::ConnectionFailed(format!("{m} [{d}]")),
            IngressError::AuthenticationFailed(m) => IngressError::AuthenticationFailed(format!("{m} [{d}]")),
            IngressError::NotFound(m) => IngressError::NotFound(format!("{m} [{d}]")),
            IngressError::Conflict(m) => IngressError::Conflict(format!("{m} [{d}]")),
            IngressError::ValidationError(m) => IngressError::ValidationError(format!("{m} [{d}]")),
            IngressError::RateLimitExceeded(m) => IngressError::RateLimitExceeded(format!("{m} [{d}]")),
            IngressError::Timeout(m) => IngressError::Timeout(format!("{m} [{d}]")),
            IngressError::NotSupported(m) => IngressError::NotSupported(format!("{m} [{d}]")),
            IngressError::IoError(e) => IngressError::InternalError(format!("io error: {e} [{d}]")),
            IngressError::SerializationError(m) => IngressError::SerializationError(format!("{m} [{d}]")),
            IngressError::BackendError(m) => IngressError::BackendError(format!("{m} [{d}]")),
            IngressError::InternalError(m) => IngressError::InternalError(format!("{m} [{d}]")),
            IngressError::AlreadyExists(m) => IngressError::AlreadyExists(format!("{m} [{d}]")),
            IngressError::PermissionDenied(m) => IngressError::PermissionDenied(format!("{m} [{d}]")),
            IngressError::Unavailable(m) => IngressError::Unavailable(format!("{m} [{d}]")),
            IngressError::Configuration(m) => IngressError::Configuration(format!("{m} [{d}]")),
        }
    }

    /// Return the canonical [`IngressErrorCode`] for this error.
    pub fn code(&self) -> IngressErrorCode {
        match self {
            IngressError::InternalError(_) | IngressError::BackendError(_) | IngressError::IoError(_) => IngressErrorCode::Internal,
            IngressError::ValidationError(_) | IngressError::SerializationError(_) => IngressErrorCode::InvalidInput,
            IngressError::NotFound(_) => IngressErrorCode::NotFound,
            IngressError::AlreadyExists(_) | IngressError::Conflict(_) => IngressErrorCode::AlreadyExists,
            IngressError::PermissionDenied(_) | IngressError::AuthenticationFailed(_) => IngressErrorCode::PermissionDenied,
            IngressError::Timeout(_) => IngressErrorCode::Timeout,
            IngressError::Unavailable(_) | IngressError::ConnectionFailed(_) | IngressError::RateLimitExceeded(_) => IngressErrorCode::Unavailable,
            IngressError::Configuration(_) | IngressError::NotSupported(_) => IngressErrorCode::Configuration,
        }
    }

    /// Returns `true` if the operation that produced this error may succeed on retry.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            IngressError::ConnectionFailed(_)
                | IngressError::RateLimitExceeded(_)
                | IngressError::Timeout(_)
                | IngressError::Unavailable(_)
        )
    }

    /// Returns `true` if this is a not-found error.
    pub fn is_not_found(&self) -> bool {
        matches!(self, IngressError::NotFound(_))
    }
}

/// Extension trait for mapping errors to inbound gateway errors.
pub trait ResultIngressExt<T> {
    /// Map any `Result<T, E>` into an [`IngressResult`] with the given context.
    fn ingress_err(self, context: impl Into<String>) -> IngressResult<T>;
}

impl<T, E: std::error::Error> ResultIngressExt<T> for Result<T, E> {
    fn ingress_err(self, context: impl Into<String>) -> IngressResult<T> {
        self.map_err(|e| IngressError::internal(context).with_details(e.to_string()))
    }
}

/// Modes for simulating failures in tests.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MockFailureMode {
    /// Fail every call with a message.
    FailAll(String),
    /// Fail calls once a counter exceeds a threshold.
    FailOverThreshold(u64),
    /// Fail calls whose ID appears in the set.
    FailSpecificIds(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: is_retryable
    #[test]
    fn test_is_retryable_returns_true_for_retryable_variants() {
        assert!(IngressError::ConnectionFailed("x".into()).is_retryable());
        assert!(IngressError::RateLimitExceeded("x".into()).is_retryable());
        assert!(IngressError::Timeout("x".into()).is_retryable());
        assert!(IngressError::Unavailable("x".into()).is_retryable());
        assert!(!IngressError::NotFound("x".into()).is_retryable());
        assert!(!IngressError::ValidationError("x".into()).is_retryable());
    }

    /// @covers: is_not_found
    #[test]
    fn test_is_not_found_returns_true_only_for_not_found_variant() {
        assert!(IngressError::NotFound("x".into()).is_not_found());
        assert!(!IngressError::InternalError("x".into()).is_not_found());
    }

    /// @covers: internal
    #[test]
    fn test_internal_creates_internal_error_code() {
        let err = IngressError::internal("test");
        assert_eq!(err.code(), IngressErrorCode::Internal);
        assert!(err.to_string().contains("test"));
    }

    /// @covers: not_found
    #[test]
    fn test_not_found_creates_not_found_error_code() {
        let err = IngressError::not_found("resource");
        assert_eq!(err.code(), IngressErrorCode::NotFound);
    }

    /// @covers: invalid_input
    #[test]
    fn test_invalid_input_creates_invalid_input_error_code() {
        let err = IngressError::invalid_input("bad");
        assert_eq!(err.code(), IngressErrorCode::InvalidInput);
    }

    /// @covers: unavailable
    #[test]
    fn test_unavailable_creates_unavailable_error_code() {
        let err = IngressError::unavailable("down");
        assert_eq!(err.code(), IngressErrorCode::Unavailable);
    }

    /// @covers: with_details
    #[test]
    fn test_with_details_appends_detail_string() {
        let err = IngressError::not_found("resource").with_details("id=42");
        assert!(err.to_string().contains("resource"));
        assert!(err.to_string().contains("[id=42]"));
    }

    /// @covers: code
    #[test]
    fn test_code_returns_correct_error_code_for_each_variant() {
        assert_eq!(IngressError::InternalError("x".into()).code(), IngressErrorCode::Internal);
        assert_eq!(IngressError::NotFound("x".into()).code(), IngressErrorCode::NotFound);
        assert_eq!(IngressError::Conflict("x".into()).code(), IngressErrorCode::AlreadyExists);
        assert_eq!(IngressError::ConnectionFailed("x".into()).code(), IngressErrorCode::Unavailable);
        assert_eq!(IngressError::NotSupported("x".into()).code(), IngressErrorCode::Configuration);
    }

    /// @covers: already_exists
    #[test]
    fn test_already_exists_creates_already_exists_error_code() {
        let err = IngressError::already_exists("dup");
        assert_eq!(err.code(), IngressErrorCode::AlreadyExists);
    }

    /// @covers: permission_denied
    #[test]
    fn test_permission_denied_creates_permission_denied_error_code() {
        let err = IngressError::permission_denied("forbidden");
        assert_eq!(err.code(), IngressErrorCode::PermissionDenied);
    }

    /// @covers: timeout
    #[test]
    fn test_timeout_creates_timeout_error_code() {
        let err = IngressError::timeout("too long");
        assert_eq!(err.code(), IngressErrorCode::Timeout);
    }

    /// @covers: configuration
    #[test]
    fn test_configuration_creates_configuration_error_code() {
        let err = IngressError::configuration("bad config");
        assert_eq!(err.code(), IngressErrorCode::Configuration);
    }
}
