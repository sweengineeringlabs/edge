//! Inbound gateway error type.

use thiserror::Error;

/// Errors produced by inbound gateway adapters.
#[derive(Debug, Error)]
pub enum IngressError {
    /// An I/O error occurred reading from the inbound source.
    #[error("inbound I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The requested resource was not found.
    #[error("not found: {path}")]
    NotFound { path: String },

    /// The inbound source is not available or not configured.
    #[error("inbound source unavailable: {reason}")]
    Unavailable { reason: String },

    /// A generic inbound error with a message.
    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingress_error_other_formats_message() {
        let e = IngressError::Other("bad input".into());
        assert_eq!(e.to_string(), "bad input");
    }

    #[test]
    fn test_ingress_error_not_found_includes_path() {
        let e = IngressError::NotFound { path: "/foo".into() };
        assert!(e.to_string().contains("/foo"));
    }
}
