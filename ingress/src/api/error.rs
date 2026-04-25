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
