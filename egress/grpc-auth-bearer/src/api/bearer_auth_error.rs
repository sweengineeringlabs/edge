//! Error variants emitted by the bearer interceptors.

/// Reasons the inbound bearer interceptor rejects a call (or the
/// outbound interceptor fails to mint a token).
#[derive(Debug, thiserror::Error)]
pub enum BearerAuthError {
    /// The `authorization` header was missing or empty.
    #[error("missing or empty authorization header")]
    MissingHeader,

    /// The header was present but not in `Bearer <jwt>` form.
    #[error("authorization header is not a Bearer token")]
    MalformedHeader,

    /// `jsonwebtoken` rejected the JWT — bad signature, wrong issuer,
    /// expired, etc.  Carries the underlying error for observability.
    #[error("invalid bearer token")]
    InvalidToken(#[source] jsonwebtoken::errors::Error),

    /// The outbound interceptor failed to encode/sign the JWT.
    #[error("failed to mint bearer token")]
    SignFailed(#[source] jsonwebtoken::errors::Error),

    /// System clock is before the Unix epoch — should never happen.
    #[error("system clock is before Unix epoch")]
    InvalidSystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: Display — MissingHeader message says "missing".
    #[test]
    fn test_missing_header_display_indicates_absent_header() {
        let s = BearerAuthError::MissingHeader.to_string();
        assert!(s.contains("missing"), "unexpected: {s}");
    }

    /// @covers: Display — MalformedHeader message names the bearer scheme.
    #[test]
    fn test_malformed_header_display_mentions_bearer_scheme() {
        let s = BearerAuthError::MalformedHeader.to_string();
        assert!(s.contains("Bearer"), "unexpected: {s}");
    }
}
