//! Runtime error and result types.

use thiserror::Error;

/// Errors that can occur during runtime manager operations.
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("start failed: {0}")]
    StartFailed(String),
    #[error("shutdown failed: {0}")]
    ShutdownFailed(String),
    #[error("bind failed: {0}")]
    BindFailed(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Result type for runtime manager operations.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_error_display_start_failed() {
        let e = RuntimeError::StartFailed("port in use".into());
        assert_eq!(e.to_string(), "start failed: port in use");
    }

    #[test]
    fn test_runtime_error_display_shutdown_failed() {
        let e = RuntimeError::ShutdownFailed("timeout".into());
        assert_eq!(e.to_string(), "shutdown failed: timeout");
    }

    #[test]
    fn test_runtime_error_display_bind_failed() {
        let e = RuntimeError::BindFailed("addr already in use".into());
        assert_eq!(e.to_string(), "bind failed: addr already in use");
    }
}
