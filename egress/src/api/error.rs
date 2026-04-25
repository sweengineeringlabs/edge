//! Outbound gateway error type.

use thiserror::Error;

/// Errors produced by outbound gateway adapters.
#[derive(Debug, Error)]
pub enum EgressError {
    /// An I/O error occurred writing to the outbound sink.
    #[error("outbound I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The outbound target rejected the request.
    #[error("outbound target error: {reason}")]
    Rejected { reason: String },

    /// The outbound adapter is not available or not configured.
    #[error("outbound adapter unavailable: {reason}")]
    Unavailable { reason: String },

    /// A generic outbound error with a message.
    #[error("{0}")]
    Other(String),
}
