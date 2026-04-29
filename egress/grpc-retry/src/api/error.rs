//! Error type for the gRPC retry decorator.

/// Errors raised while loading config or building the retry layer.
///
/// Runtime errors from the wrapped [`GrpcOutbound`] flow through
/// as the inner crate's `GrpcOutboundError`; they are not
/// re-wrapped here.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Config TOML didn't parse as the expected schema.
    /// Wraps the underlying `toml::de::Error` message, which
    /// names the missing or unknown field when that's the cause.
    #[error("swe_edge_egress_grpc_retry: config parse failed — {0}")]
    ParseFailed(String),

    /// A configured numeric value is outside its valid range
    /// (e.g. `backoff_multiplier <= 0.0`).
    #[error("swe_edge_egress_grpc_retry: invalid config — {0}")]
    InvalidConfig(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: Error
    #[test]
    fn test_parse_failed_display_names_crate_and_reason() {
        let err = Error::ParseFailed("missing field `max_attempts`".into());
        let s   = err.to_string();
        assert!(s.contains("swe_edge_egress_grpc_retry"));
        assert!(s.contains("max_attempts"));
    }

    /// @covers: Error
    #[test]
    fn test_invalid_config_display_includes_crate_name() {
        let err = Error::InvalidConfig("backoff_multiplier must be > 0".into());
        let s   = err.to_string();
        assert!(s.contains("swe_edge_egress_grpc_retry"));
        assert!(s.contains("backoff_multiplier"));
    }
}
