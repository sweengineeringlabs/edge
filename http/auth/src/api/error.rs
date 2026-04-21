//! Error type for the auth middleware.

/// Errors raised by the auth middleware.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Config TOML didn't parse as the expected schema.
    #[error("swe_http_auth: config parse failed — {0}")]
    ParseFailed(String),

    /// Config references an env var that isn't set. Includes
    /// the missing var name so operators know what to export.
    /// This fails at `Builder::build()` (or equivalent config
    /// realization) — the middleware refuses to construct with
    /// a dangling credential reference.
    #[error("swe_http_auth: required env var {name} is not set")]
    MissingEnvVar {
        /// Name of the missing env var.
        name: String,
    },

    /// Unknown or unsupported `kind` in config. The config
    /// schema lists the accepted values.
    #[error("swe_http_auth: unsupported auth kind {kind:?} — expected one of: none, bearer, basic, header")]
    UnsupportedKind {
        /// The offending kind string.
        kind: String,
    },

    /// Middleware behavior not yet implemented (scaffold phase).
    #[error("swe_http_auth: not implemented — {0}")]
    NotImplemented(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: Error
    #[test]
    fn test_not_implemented_display_includes_crate_name() {
        let err = Error::NotImplemented("builder");
        assert!(err.to_string().contains("swe_http_auth"));
    }

    /// @covers: Error
    #[test]
    fn test_missing_env_var_names_the_variable() {
        let err = Error::MissingEnvVar { name: "EDGE_API_TOKEN".into() };
        let s = err.to_string();
        assert!(s.contains("EDGE_API_TOKEN"));
    }

    /// @covers: Error
    #[test]
    fn test_unsupported_kind_names_valid_options() {
        let err = Error::UnsupportedKind { kind: "digest".into() };
        let s = err.to_string();
        assert!(s.contains("digest"));
        assert!(s.contains("bearer"));
        assert!(s.contains("basic"));
    }

    /// @covers: Error
    #[test]
    fn test_parse_failed_display_names_crate_and_reason() {
        let err = Error::ParseFailed("missing field".into());
        let s = err.to_string();
        assert!(s.contains("swe_http_auth"));
        assert!(s.contains("missing field"));
    }
}
