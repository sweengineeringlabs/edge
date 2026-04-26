//! Integration tests for `core::identity::pkcs12_http_tls::Pkcs12HttpTls`.
//!
//! `Pkcs12HttpTls` is `pub(crate)`. Integration tests verify its contract
//! through the public `Builder::with_config(TlsConfig::Pkcs12 { .. })` path:
//!
//! - A missing PKCS12 file causes `build()` to return `Error::FileReadFailed`.
//! - An unset `password_env` causes `build()` to return `Error::MissingEnvVar`.
//! - A set (any value) `password_env` with a missing file returns
//!   `Error::FileReadFailed` (password is resolved before file read).
//! - A file that exists but is not valid PKCS12 data causes `apply_to()`
//!   to return `Error::InvalidCertificate { format: "pkcs12", .. }`.

use swe_edge_egress_tls::{Builder, Error, TlsApplier, TlsConfig};

// ---------------------------------------------------------------------------
// Pkcs12HttpTls::load — missing file errors surface at build time
// ---------------------------------------------------------------------------

/// A missing PKCS12 file (no password configured) must return
/// `Error::FileReadFailed` at `build()` time.
#[test]
fn test_pkcs12_missing_file_no_password_returns_file_read_failed() {
    let cfg = TlsConfig::Pkcs12 {
        path: "/path/does/not/exist/cert.p12".into(),
        password_env: None,
    };
    let err = Builder::with_config(cfg).build().unwrap_err();
    match err {
        Error::FileReadFailed { path, .. } => {
            assert!(
                path.contains("does/not/exist"),
                "FileReadFailed path must embed the configured path; got: {path}"
            );
        }
        other => panic!("expected FileReadFailed, got: {other:?}"),
    }
}

/// The `FileReadFailed` error must include the configured path so operators
/// can immediately identify the misconfigured file.
#[test]
fn test_pkcs12_file_read_failed_contains_configured_path() {
    let cfg = TlsConfig::Pkcs12 {
        path: "/very/specific/cert.p12".into(),
        password_env: None,
    };
    let msg = Builder::with_config(cfg).build().unwrap_err().to_string();
    assert!(
        msg.contains("cert.p12"),
        "error message must contain the configured filename; got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Pkcs12HttpTls: password_env resolution happens before file read
// ---------------------------------------------------------------------------

/// When `password_env` names an env var that is NOT set, `build()` must
/// return `Error::MissingEnvVar` immediately — before reading the file.
/// (The path is deliberately irrelevant — we want to confirm the env var
/// check comes first.)
#[test]
fn test_pkcs12_missing_password_env_precedes_file_read() {
    let env_name = "SWE_IT_PKCS12_PW_ABSENT_04";
    std::env::remove_var(env_name);
    let cfg = TlsConfig::Pkcs12 {
        path: "irrelevant.p12".into(),
        password_env: Some(env_name.into()),
    };
    let err = Builder::with_config(cfg).build().unwrap_err();
    match err {
        Error::MissingEnvVar { name } => {
            assert_eq!(name, env_name, "MissingEnvVar must name the exact env var");
        }
        other => panic!("expected MissingEnvVar, got: {other:?}"),
    }
}

/// When `password_env` IS set but the file is missing, `build()` must
/// return `Error::FileReadFailed` (the password was resolved, but then
/// the file read failed).
#[test]
fn test_pkcs12_set_password_env_but_missing_file_returns_file_read_failed() {
    let env_name = "SWE_IT_PKCS12_PW_SET_FILE_MISSING_05";
    std::env::set_var(env_name, "some-password");
    let cfg = TlsConfig::Pkcs12 {
        path: "/path/does/not/exist/cert.p12".into(),
        password_env: Some(env_name.into()),
    };
    let err = Builder::with_config(cfg).build().unwrap_err();
    assert!(
        matches!(err, Error::FileReadFailed { .. }),
        "with password set but missing file, must return FileReadFailed; got: {err:?}"
    );
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Pkcs12HttpTls::identity — invalid DER bytes returns InvalidCertificate
// ---------------------------------------------------------------------------

/// A file that exists but is not valid PKCS12 DER data must cause
/// `apply_to()` to return `Error::InvalidCertificate { format: "pkcs12", .. }`.
/// The DER bytes are read at `build()` time, but `identity()` (called from
/// `apply_to()`) is where the parsing occurs.
#[test]
fn test_pkcs12_invalid_content_returns_invalid_certificate_on_apply_to() {
    let tmpdir = tempfile::tempdir().unwrap();
    let path = tmpdir.path().join("fake.p12");
    std::fs::write(&path, b"not-valid-pkcs12-der-content").unwrap();

    let cfg = TlsConfig::Pkcs12 {
        path: path.to_str().unwrap().replace('\\', "/"),
        password_env: None,
    };
    // build() succeeds: file exists and is readable.
    let layer = Builder::with_config(cfg).build().expect("build must succeed for existing file");

    // apply_to() calls identity() → from_pkcs12_der() → InvalidCertificate.
    let err = layer
        .apply_to(reqwest::Client::builder())
        .unwrap_err();
    match err {
        Error::InvalidCertificate { format, reason } => {
            assert_eq!(format, "pkcs12", "format must be 'pkcs12'; got: {format}");
            assert!(!reason.is_empty(), "reason must not be empty; got: {reason}");
        }
        other => panic!("expected InvalidCertificate, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Pkcs12HttpTls: bytes read eagerly at build, not at apply_to
// ---------------------------------------------------------------------------

/// Deleting the PKCS12 file AFTER `build()` must not change the error type —
/// `apply_to()` must produce `InvalidCertificate` (bytes already loaded),
/// not `FileReadFailed`.
#[test]
fn test_pkcs12_bytes_read_eagerly_at_build_not_at_apply_to() {
    let tmpdir = tempfile::tempdir().unwrap();
    let path = tmpdir.path().join("ephemeral.p12");
    std::fs::write(&path, b"not-pkcs12-content").unwrap();

    let cfg = TlsConfig::Pkcs12 {
        path: path.to_str().unwrap().replace('\\', "/"),
        password_env: None,
    };
    let layer = Builder::with_config(cfg).build().expect("build must succeed");

    // Delete the file after build.
    std::fs::remove_file(&path).unwrap();

    // apply_to must produce InvalidCertificate, not FileReadFailed.
    let err = layer.apply_to(reqwest::Client::builder()).unwrap_err();
    assert!(
        matches!(err, Error::InvalidCertificate { .. }),
        "post-deletion error must be InvalidCertificate, not FileReadFailed; got: {err:?}"
    );
}
