//! Error type for the retry middleware.

/// Errors raised by the retry middleware.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Config TOML didn't parse as the expected schema.
    /// Wraps the underlying `toml::de::Error` message, which
    /// names the missing or unknown field when that's the cause.
    #[error("swe_http_retry: config parse failed — {0}")]
    ParseFailed(String),

    /// Middleware behavior not yet implemented (scaffold phase).
    /// Replaced with richer variants when the real impl lands.
    #[error("swe_http_retry: not implemented — {0}")]
    NotImplemented(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: Error
    #[test]
    fn test_not_implemented_display_includes_crate_name() {
        let err = Error::NotImplemented("builder");
        let s = err.to_string();
        assert!(s.contains("swe_http_retry"));
        assert!(s.contains("builder"));
    }

    /// @covers: Error
    #[test]
    fn test_parse_failed_display_names_crate_and_reason() {
        let err = Error::ParseFailed("missing field `max_retries`".into());
        let s = err.to_string();
        assert!(s.contains("swe_http_retry"));
        assert!(s.contains("max_retries"));
    }
}
