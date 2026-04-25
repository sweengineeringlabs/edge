//! Outbound gateway error type.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result type for outbound gateway operations.
pub type EgressResult<T> = Result<T, EgressError>;

/// Standard error codes for outbound gateway operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EgressErrorCode {
    Internal,
    InvalidInput,
    NotFound,
    AlreadyExists,
    PermissionDenied,
    Timeout,
    Unavailable,
    Configuration,
}

/// Comprehensive error type for outbound gateway operations.
#[derive(Debug, Error)]
pub enum EgressError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("validation error: {0}")]
    ValidationError(String),

    #[error("rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("not supported: {0}")]
    NotSupported(String),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("backend error: {0}")]
    BackendError(String),

    #[error("internal error: {0}")]
    InternalError(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("unavailable: {0}")]
    Unavailable(String),

    #[error("configuration error: {0}")]
    Configuration(String),
}

impl EgressError {
    /// Create an outbound error from a code and message.
    pub fn new(code: EgressErrorCode, message: impl Into<String>) -> Self {
        let msg = message.into();
        match code {
            EgressErrorCode::Internal => EgressError::InternalError(msg),
            EgressErrorCode::InvalidInput => EgressError::ValidationError(msg),
            EgressErrorCode::NotFound => EgressError::NotFound(msg),
            EgressErrorCode::AlreadyExists => EgressError::AlreadyExists(msg),
            EgressErrorCode::PermissionDenied => EgressError::PermissionDenied(msg),
            EgressErrorCode::Timeout => EgressError::Timeout(msg),
            EgressErrorCode::Unavailable => EgressError::Unavailable(msg),
            EgressErrorCode::Configuration => EgressError::Configuration(msg),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(EgressErrorCode::Internal, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(EgressErrorCode::NotFound, message)
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(EgressErrorCode::InvalidInput, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(EgressErrorCode::Unavailable, message)
    }

    pub fn already_exists(message: impl Into<String>) -> Self {
        Self::new(EgressErrorCode::AlreadyExists, message)
    }

    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(EgressErrorCode::PermissionDenied, message)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(EgressErrorCode::Timeout, message)
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::new(EgressErrorCode::Configuration, message)
    }

    pub fn with_details(self, details: impl Into<String>) -> Self {
        let d = details.into();
        match self {
            EgressError::ConnectionFailed(m) => EgressError::ConnectionFailed(format!("{m} [{d}]")),
            EgressError::AuthenticationFailed(m) => EgressError::AuthenticationFailed(format!("{m} [{d}]")),
            EgressError::NotFound(m) => EgressError::NotFound(format!("{m} [{d}]")),
            EgressError::Conflict(m) => EgressError::Conflict(format!("{m} [{d}]")),
            EgressError::ValidationError(m) => EgressError::ValidationError(format!("{m} [{d}]")),
            EgressError::RateLimitExceeded(m) => EgressError::RateLimitExceeded(format!("{m} [{d}]")),
            EgressError::Timeout(m) => EgressError::Timeout(format!("{m} [{d}]")),
            EgressError::NotSupported(m) => EgressError::NotSupported(format!("{m} [{d}]")),
            EgressError::IoError(e) => EgressError::InternalError(format!("io error: {e} [{d}]")),
            EgressError::SerializationError(m) => EgressError::SerializationError(format!("{m} [{d}]")),
            EgressError::BackendError(m) => EgressError::BackendError(format!("{m} [{d}]")),
            EgressError::InternalError(m) => EgressError::InternalError(format!("{m} [{d}]")),
            EgressError::AlreadyExists(m) => EgressError::AlreadyExists(format!("{m} [{d}]")),
            EgressError::PermissionDenied(m) => EgressError::PermissionDenied(format!("{m} [{d}]")),
            EgressError::Unavailable(m) => EgressError::Unavailable(format!("{m} [{d}]")),
            EgressError::Configuration(m) => EgressError::Configuration(format!("{m} [{d}]")),
        }
    }

    pub fn code(&self) -> EgressErrorCode {
        match self {
            EgressError::InternalError(_) | EgressError::BackendError(_) | EgressError::IoError(_) => EgressErrorCode::Internal,
            EgressError::ValidationError(_) | EgressError::SerializationError(_) => EgressErrorCode::InvalidInput,
            EgressError::NotFound(_) => EgressErrorCode::NotFound,
            EgressError::AlreadyExists(_) | EgressError::Conflict(_) => EgressErrorCode::AlreadyExists,
            EgressError::PermissionDenied(_) | EgressError::AuthenticationFailed(_) => EgressErrorCode::PermissionDenied,
            EgressError::Timeout(_) => EgressErrorCode::Timeout,
            EgressError::Unavailable(_) | EgressError::ConnectionFailed(_) | EgressError::RateLimitExceeded(_) => EgressErrorCode::Unavailable,
            EgressError::Configuration(_) | EgressError::NotSupported(_) => EgressErrorCode::Configuration,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            EgressError::ConnectionFailed(_)
                | EgressError::RateLimitExceeded(_)
                | EgressError::Timeout(_)
                | EgressError::Unavailable(_)
        )
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, EgressError::NotFound(_))
    }
}

/// Extension trait for mapping errors to outbound gateway errors.
pub trait ResultEgressExt<T> {
    fn egress_err(self, context: impl Into<String>) -> EgressResult<T>;
}

impl<T, E: std::error::Error> ResultEgressExt<T> for Result<T, E> {
    fn egress_err(self, context: impl Into<String>) -> EgressResult<T> {
        self.map_err(|e| EgressError::internal(context).with_details(e.to_string()))
    }
}

/// Modes for simulating payment failures in tests.
#[allow(dead_code, clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MockFailureMode {
    FailAllPayments(String),
    FailOverAmount(i64),
    FailPaymentIds(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: is_retryable
    #[test]
    fn test_is_retryable_returns_true_for_retryable_variants() {
        assert!(EgressError::ConnectionFailed("x".into()).is_retryable());
        assert!(EgressError::RateLimitExceeded("x".into()).is_retryable());
        assert!(EgressError::Timeout("x".into()).is_retryable());
        assert!(EgressError::Unavailable("x".into()).is_retryable());
        assert!(!EgressError::NotFound("x".into()).is_retryable());
        assert!(!EgressError::ValidationError("x".into()).is_retryable());
    }

    /// @covers: is_not_found
    #[test]
    fn test_is_not_found_returns_true_only_for_not_found_variant() {
        assert!(EgressError::NotFound("x".into()).is_not_found());
        assert!(!EgressError::InternalError("x".into()).is_not_found());
    }

    /// @covers: internal
    #[test]
    fn test_internal_creates_internal_error_code() {
        let err = EgressError::internal("test");
        assert_eq!(err.code(), EgressErrorCode::Internal);
        assert!(err.to_string().contains("test"));
    }

    /// @covers: not_found
    #[test]
    fn test_not_found_creates_not_found_error_code() {
        let err = EgressError::not_found("resource");
        assert_eq!(err.code(), EgressErrorCode::NotFound);
    }

    /// @covers: invalid_input
    #[test]
    fn test_invalid_input_creates_invalid_input_error_code() {
        let err = EgressError::invalid_input("bad");
        assert_eq!(err.code(), EgressErrorCode::InvalidInput);
    }

    /// @covers: unavailable
    #[test]
    fn test_unavailable_creates_unavailable_error_code() {
        let err = EgressError::unavailable("down");
        assert_eq!(err.code(), EgressErrorCode::Unavailable);
    }

    /// @covers: with_details
    #[test]
    fn test_with_details_appends_detail_string() {
        let err = EgressError::not_found("resource").with_details("id=42");
        assert!(err.to_string().contains("resource"));
        assert!(err.to_string().contains("[id=42]"));
    }

    /// @covers: code
    #[test]
    fn test_code_returns_correct_error_code_for_each_variant() {
        assert_eq!(EgressError::InternalError("x".into()).code(), EgressErrorCode::Internal);
        assert_eq!(EgressError::NotFound("x".into()).code(), EgressErrorCode::NotFound);
        assert_eq!(EgressError::Conflict("x".into()).code(), EgressErrorCode::AlreadyExists);
        assert_eq!(EgressError::ConnectionFailed("x".into()).code(), EgressErrorCode::Unavailable);
        assert_eq!(EgressError::NotSupported("x".into()).code(), EgressErrorCode::Configuration);
    }

    /// @covers: already_exists
    #[test]
    fn test_already_exists_creates_already_exists_error_code() {
        let err = EgressError::already_exists("dup");
        assert_eq!(err.code(), EgressErrorCode::AlreadyExists);
    }

    /// @covers: permission_denied
    #[test]
    fn test_permission_denied_creates_permission_denied_error_code() {
        let err = EgressError::permission_denied("forbidden");
        assert_eq!(err.code(), EgressErrorCode::PermissionDenied);
    }

    /// @covers: timeout
    #[test]
    fn test_timeout_creates_timeout_error_code() {
        let err = EgressError::timeout("too long");
        assert_eq!(err.code(), EgressErrorCode::Timeout);
    }

    /// @covers: configuration
    #[test]
    fn test_configuration_creates_configuration_error_code() {
        let err = EgressError::configuration("bad config");
        assert_eq!(err.code(), EgressErrorCode::Configuration);
    }
}
