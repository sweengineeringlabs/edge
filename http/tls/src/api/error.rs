//! Error type for the tls middleware.

/// Errors raised by swe_edge_http_tls.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Config TOML didn't parse as the expected schema.
    #[error("swe_edge_http_tls: config parse failed — {0}")]
    ParseFailed(String),

    /// Config references an env var that isn't set.
    #[error("swe_edge_http_tls: required env var {name} is not set")]
    MissingEnvVar {
        /// The missing env-var name.
        name: String,
    },

    /// Certificate / key file couldn't be read from disk. Wraps
    /// the underlying IO error message for operators to
    /// diagnose (wrong path, permissions, etc.).
    #[error("swe_edge_http_tls: could not read file {path:?} — {reason}")]
    FileReadFailed {
        /// Path the config referenced.
        path: String,
        /// Underlying I/O error.
        reason: String,
    },

    /// Certificate / key data is present but not parseable as
    /// the declared format. E.g. PKCS12 with wrong password,
    /// PEM with malformed blocks.
    #[error("swe_edge_http_tls: invalid {format} data — {reason}")]
    InvalidCertificate {
        /// Format we tried to parse as (pkcs12, pem).
        format: &'static str,
        /// Underlying parse error.
        reason: String,
    },

    /// Builder hasn't been implemented yet (scaffold phase).
    #[error("swe_edge_http_tls: not implemented — {0}")]
    NotImplemented(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: Error
    #[test]
    fn test_parse_failed_display_names_crate() {
        let err = Error::ParseFailed("bad".into());
        assert!(err.to_string().contains("swe_edge_http_tls"));
    }

    /// @covers: Error
    #[test]
    fn test_missing_env_var_names_variable() {
        let err = Error::MissingEnvVar {
            name: "EDGE_TLS_PASSWORD".into(),
        };
        assert!(err.to_string().contains("EDGE_TLS_PASSWORD"));
    }

    /// @covers: Error
    #[test]
    fn test_file_read_failed_includes_path_and_reason() {
        let err = Error::FileReadFailed {
            path: "/etc/cert.p12".into(),
            reason: "Permission denied".into(),
        };
        let s = err.to_string();
        assert!(s.contains("/etc/cert.p12"));
        assert!(s.contains("Permission denied"));
    }

    /// @covers: Error
    #[test]
    fn test_invalid_certificate_names_format() {
        let err = Error::InvalidCertificate {
            format: "pkcs12",
            reason: "wrong password".into(),
        };
        let s = err.to_string();
        assert!(s.contains("pkcs12"));
        assert!(s.contains("wrong password"));
    }
}
