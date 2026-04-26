//! Integration tests for `swe_edge_egress_tls::Error`.
//!
//! Covers: `Error::ParseFailed`, `Error::MissingEnvVar`, `Error::FileReadFailed`,
//! `Error::InvalidCertificate`, `Error::NotImplemented` — Display messages
//! must be actionable: name the crate, embed the payload, be non-empty.

use swe_edge_egress_tls::Error;

// ---------------------------------------------------------------------------
// Error::ParseFailed
// ---------------------------------------------------------------------------

#[test]
fn test_parse_failed_display_names_the_crate() {
    let msg = Error::ParseFailed("unknown field `jks`".to_string()).to_string();
    assert!(
        msg.contains("swe_edge_egress_tls"),
        "ParseFailed Display must name the crate; got: {msg}"
    );
}

#[test]
fn test_parse_failed_display_contains_supplied_reason() {
    let msg = Error::ParseFailed("missing field `path`".to_string()).to_string();
    assert!(msg.contains("path"), "ParseFailed must embed the reason; got: {msg}");
}

#[test]
fn test_parse_failed_is_debug_printable() {
    let _ = format!("{:?}", Error::ParseFailed("bad".to_string()));
}

#[test]
fn test_parse_failed_is_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(Error::ParseFailed("oops".to_string()));
    assert!(!err.to_string().is_empty());
}

// ---------------------------------------------------------------------------
// Error::MissingEnvVar
// ---------------------------------------------------------------------------

/// Display must contain the missing env var name so the operator knows
/// exactly which variable to set to fix the issue.
#[test]
fn test_missing_env_var_display_contains_var_name() {
    let msg = Error::MissingEnvVar {
        name: "EDGE_MTLS_PASSWORD".to_string(),
    }
    .to_string();
    assert!(
        msg.contains("EDGE_MTLS_PASSWORD"),
        "MissingEnvVar must name the variable; got: {msg}"
    );
}

/// Display must name the crate.
#[test]
fn test_missing_env_var_display_names_the_crate() {
    let msg = Error::MissingEnvVar {
        name: "ANY_VAR".to_string(),
    }
    .to_string();
    assert!(
        msg.contains("swe_edge_egress_tls"),
        "MissingEnvVar Display must name the crate; got: {msg}"
    );
}

#[test]
fn test_missing_env_var_is_debug_printable() {
    let _ = format!("{:?}", Error::MissingEnvVar { name: "X".to_string() });
}

#[test]
fn test_missing_env_var_is_std_error() {
    let err: Box<dyn std::error::Error> =
        Box::new(Error::MissingEnvVar { name: "Y".to_string() });
    assert!(!err.to_string().is_empty());
}

// ---------------------------------------------------------------------------
// Error::FileReadFailed
// ---------------------------------------------------------------------------

/// Display must include the path so the operator knows which file to check.
#[test]
fn test_file_read_failed_display_contains_path() {
    let msg = Error::FileReadFailed {
        path: "/etc/certs/client.p12".to_string(),
        reason: "No such file or directory".to_string(),
    }
    .to_string();
    assert!(
        msg.contains("/etc/certs/client.p12"),
        "FileReadFailed must embed the path; got: {msg}"
    );
}

/// Display must include the reason so the operator knows whether it was a
/// permissions issue, a missing file, or something else.
#[test]
fn test_file_read_failed_display_contains_reason() {
    let msg = Error::FileReadFailed {
        path: "/any/path".to_string(),
        reason: "Permission denied (os error 13)".to_string(),
    }
    .to_string();
    assert!(
        msg.contains("Permission denied"),
        "FileReadFailed must embed the reason; got: {msg}"
    );
}

/// Display must name the crate.
#[test]
fn test_file_read_failed_display_names_the_crate() {
    let msg = Error::FileReadFailed {
        path: "p".to_string(),
        reason: "r".to_string(),
    }
    .to_string();
    assert!(msg.contains("swe_edge_egress_tls"), "FileReadFailed Display must name the crate; got: {msg}");
}

#[test]
fn test_file_read_failed_is_debug_printable() {
    let _ = format!(
        "{:?}",
        Error::FileReadFailed {
            path: "/p".to_string(),
            reason: "r".to_string()
        }
    );
}

#[test]
fn test_file_read_failed_is_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(Error::FileReadFailed {
        path: "/p".to_string(),
        reason: "r".to_string(),
    });
    assert!(!err.to_string().is_empty());
}

// ---------------------------------------------------------------------------
// Error::InvalidCertificate
// ---------------------------------------------------------------------------

/// Display must name the format so the operator knows which parser failed.
#[test]
fn test_invalid_certificate_display_names_format() {
    let msg = Error::InvalidCertificate {
        format: "pkcs12",
        reason: "wrong password".to_string(),
    }
    .to_string();
    assert!(msg.contains("pkcs12"), "InvalidCertificate must name the format; got: {msg}");
}

/// Display must include the reason (e.g. "wrong password") for diagnosability.
#[test]
fn test_invalid_certificate_display_contains_reason() {
    let msg = Error::InvalidCertificate {
        format: "pem",
        reason: "no CERTIFICATE block found".to_string(),
    }
    .to_string();
    assert!(
        msg.contains("no CERTIFICATE block found"),
        "InvalidCertificate must embed the reason; got: {msg}"
    );
}

/// Display must name the crate.
#[test]
fn test_invalid_certificate_display_names_the_crate() {
    let msg = Error::InvalidCertificate {
        format: "pem",
        reason: "bad".to_string(),
    }
    .to_string();
    assert!(msg.contains("swe_edge_egress_tls"), "InvalidCertificate Display must name the crate; got: {msg}");
}

#[test]
fn test_invalid_certificate_is_debug_printable() {
    let _ = format!(
        "{:?}",
        Error::InvalidCertificate {
            format: "pem",
            reason: "bad".to_string()
        }
    );
}

// ---------------------------------------------------------------------------
// Error::NotImplemented
// ---------------------------------------------------------------------------

#[test]
fn test_not_implemented_display_is_non_empty() {
    assert!(!Error::NotImplemented("test").to_string().is_empty());
}

#[test]
fn test_not_implemented_display_names_the_crate() {
    let msg = Error::NotImplemented("some feature").to_string();
    assert!(msg.contains("swe_edge_egress_tls"), "NotImplemented must name the crate; got: {msg}");
}

#[test]
fn test_not_implemented_display_contains_label() {
    let msg = Error::NotImplemented("grpc_identity").to_string();
    assert!(msg.contains("grpc_identity"), "NotImplemented must embed the label; got: {msg}");
}

// ---------------------------------------------------------------------------
// All five variants are distinct
// ---------------------------------------------------------------------------

/// All five error variants for the same conceptual payload must produce
/// distinct display strings — callers must be able to distinguish them.
#[test]
fn test_all_error_variants_have_distinct_display_messages() {
    let messages: Vec<String> = vec![
        Error::ParseFailed("x".into()).to_string(),
        Error::MissingEnvVar { name: "x".into() }.to_string(),
        Error::FileReadFailed {
            path: "x".into(),
            reason: "x".into(),
        }
        .to_string(),
        Error::InvalidCertificate {
            format: "pem",
            reason: "x".into(),
        }
        .to_string(),
        Error::NotImplemented("x").to_string(),
    ];
    // No two messages should be identical.
    for (i, a) in messages.iter().enumerate() {
        for (j, b) in messages.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "variants {i} and {j} must have distinct display messages");
            }
        }
    }
}
