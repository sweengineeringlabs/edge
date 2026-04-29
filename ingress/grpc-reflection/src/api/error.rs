//! Error types for the reflection crate.

use thiserror::Error;

/// Errors raised while encoding/decoding reflection wire messages.
///
/// The reflection RPC itself never returns these on the wire — it
/// translates them into a structured `ErrorResponse` so grpcurl can
/// keep its bidi stream open.  They are surfaced from the codec to
/// the dispatcher and are useful in tests.
#[derive(Debug, Error)]
pub enum ReflectionError {
    /// Wire-format error in an inbound `ServerReflectionRequest` —
    /// truncated body, malformed varint, etc.
    #[error("malformed reflection request: {0}")]
    Malformed(String),

    /// Inbound request was syntactically valid but its `message_request`
    /// oneof carried a field number this server does not understand.
    #[error("unknown reflection request field: {0}")]
    UnknownRequest(u32),
}
