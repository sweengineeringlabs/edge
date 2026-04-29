//! Error type for the gRPC breaker decorator.

/// Errors raised while loading config or building the breaker.
///
/// Runtime errors from the wrapped [`GrpcOutbound`] flow through
/// as the inner crate's `GrpcOutboundError`; they are not
/// re-wrapped here.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Config TOML didn't parse as the expected schema.
    #[error("swe_edge_egress_grpc_breaker: config parse failed — {0}")]
    ParseFailed(String),

    /// A configured value is invalid (e.g. zero failure_threshold).
    #[error("swe_edge_egress_grpc_breaker: invalid config — {0}")]
    InvalidConfig(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: Error
    #[test]
    fn test_parse_failed_display_names_crate_and_reason() {
        let err = Error::ParseFailed("missing field `failure_threshold`".into());
        let s   = err.to_string();
        assert!(s.contains("swe_edge_egress_grpc_breaker"));
        assert!(s.contains("failure_threshold"));
    }

    /// @covers: Error
    #[test]
    fn test_invalid_config_display_includes_crate_name() {
        let err = Error::InvalidConfig("failure_threshold must be >= 1".into());
        let s   = err.to_string();
        assert!(s.contains("swe_edge_egress_grpc_breaker"));
        assert!(s.contains("failure_threshold"));
    }
}
