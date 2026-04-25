//! Error type for the rate middleware.

/// Errors raised by the rate middleware.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Config TOML didn't parse as the expected schema.
    #[error("swe_edge_http_rate: config parse failed — {0}")]
    ParseFailed(String),

    /// Middleware behavior not yet implemented (scaffold phase).
    #[error("swe_edge_http_rate: not implemented — {0}")]
    NotImplemented(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: Error
    #[test]
    fn test_not_implemented_display_includes_crate_name() {
        let err = Error::NotImplemented("builder");
        assert!(err.to_string().contains("swe_edge_http_rate"));
    }

    /// @covers: Error
    #[test]
    fn test_parse_failed_display_names_crate_and_reason() {
        let err = Error::ParseFailed("missing field".into());
        let s = err.to_string();
        assert!(s.contains("swe_edge_http_rate"));
        assert!(s.contains("missing field"));
    }
}
