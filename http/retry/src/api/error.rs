//! Error type for the retry middleware.

/// Errors raised by the retry middleware. Scaffold phase: only
/// [`Error::NotImplemented`]. Real variants land with the impl.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Placeholder — the middleware's public API exists but the
    /// underlying behavior isn't implemented yet. Replaced with
    /// specific variants when the impl lands.
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
}
