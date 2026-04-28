//! Error type returned by the mTLS interceptor when access is denied.

/// Reasons the mTLS interceptor rejects a call.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MtlsAuthError {
    /// The request reached the interceptor without any peer-cert
    /// fingerprint set — typically a plaintext / TLS-only conn.
    #[error("no mTLS peer identity present in request metadata")]
    MissingIdentity,

    /// The peer's CN was not on the configured allowlist.
    #[error("peer CN '{0}' is not on the allowlist")]
    DisallowedCn(String),

    /// None of the peer's DNS SANs matched the configured allowlist.
    #[error("none of the peer's DNS SANs match the allowlist")]
    DisallowedSan,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: MtlsAuthError::MissingIdentity — display message.
    #[test]
    fn test_missing_identity_display_message_mentions_no_identity() {
        let err = MtlsAuthError::MissingIdentity;
        let s = err.to_string();
        assert!(s.contains("no mTLS peer identity"), "unexpected: {s}");
    }

    /// @covers: MtlsAuthError::DisallowedCn — display includes the CN.
    #[test]
    fn test_disallowed_cn_display_includes_offending_cn() {
        let err = MtlsAuthError::DisallowedCn("evil.svc".into());
        assert!(err.to_string().contains("evil.svc"));
    }
}
