//! Integration tests for `TlsConfig` public surface.
//!
//! `TlsConfig` is a public enum with three variants: `None`, `Pem { path }`,
//! and `Pkcs12 { path, password_env }`. `from_config` and `swe_default` are
//! both `pub` (unlike the other crates where they are `pub(crate)`).
//!
//! Tests verify:
//! - Struct/enum variants are constructible via direct literal syntax.
//! - `from_config` parses valid TOML for each variant.
//! - `from_config` rejects unknown `kind` values and inline `password` fields.
//! - `swe_default()` always returns `TlsConfig::None`.
//! - Values flow unchanged through `Builder::with_config`.

use swe_http_tls::{Builder, Error, TlsConfig};

// ---------------------------------------------------------------------------
// Direct variant construction
// ---------------------------------------------------------------------------

/// All three variants must be constructible via literal syntax. A rename
/// or removal causes this test to fail to compile, catching API breaks.
#[test]
fn test_all_tls_config_variants_are_constructible() {
    let _none = TlsConfig::None;
    let _pem = TlsConfig::Pem {
        path: "/etc/cert.pem".to_string(),
    };
    let _pkcs12_no_pw = TlsConfig::Pkcs12 {
        path: "/etc/cert.p12".to_string(),
        password_env: None,
    };
    let _pkcs12_with_pw = TlsConfig::Pkcs12 {
        path: "/etc/cert.p12".to_string(),
        password_env: Some("EDGE_TLS_PW".to_string()),
    };
}

/// `TlsConfig` must be `Clone`.
#[test]
fn test_tls_config_is_clone() {
    let cfg = TlsConfig::Pem {
        path: "/some/path.pem".to_string(),
    };
    let cloned = cfg.clone();
    assert!(matches!(cloned, TlsConfig::Pem { .. }));
}

// ---------------------------------------------------------------------------
// TlsConfig::from_config — parses all valid TOML forms
// ---------------------------------------------------------------------------

/// `kind = "none"` must parse as `TlsConfig::None`.
#[test]
fn test_from_config_parses_none_variant() {
    let cfg = TlsConfig::from_config(r#"kind = "none""#).expect("parses");
    assert!(matches!(cfg, TlsConfig::None));
}

/// `kind = "pem"` with a path must parse as `TlsConfig::Pem`.
#[test]
fn test_from_config_parses_pem_variant() {
    let cfg = TlsConfig::from_config(
        r#"
            kind = "pem"
            path = "/etc/client-combined.pem"
        "#,
    )
    .expect("parses");
    match cfg {
        TlsConfig::Pem { path } => assert_eq!(path, "/etc/client-combined.pem"),
        other => panic!("expected Pem, got: {other:?}"),
    }
}

/// `kind = "pkcs12"` with path and `password_env` must parse as
/// `TlsConfig::Pkcs12` with both fields set.
#[test]
fn test_from_config_parses_pkcs12_with_password_env() {
    let cfg = TlsConfig::from_config(
        r#"
            kind = "pkcs12"
            path = "/etc/client.p12"
            password_env = "EDGE_TLS_PASSWORD"
        "#,
    )
    .expect("parses");
    match cfg {
        TlsConfig::Pkcs12 { path, password_env } => {
            assert_eq!(path, "/etc/client.p12");
            assert_eq!(password_env.as_deref(), Some("EDGE_TLS_PASSWORD"));
        }
        other => panic!("expected Pkcs12, got: {other:?}"),
    }
}

/// `kind = "pkcs12"` without `password_env` must parse as `Pkcs12` with
/// `password_env = None`.
#[test]
fn test_from_config_parses_pkcs12_without_password_env() {
    let cfg = TlsConfig::from_config(
        r#"
            kind = "pkcs12"
            path = "/etc/client.p12"
        "#,
    )
    .expect("parses");
    match cfg {
        TlsConfig::Pkcs12 { path, password_env } => {
            assert_eq!(path, "/etc/client.p12");
            assert!(password_env.is_none(), "password_env must be None when omitted");
        }
        other => panic!("expected Pkcs12, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// TlsConfig::from_config — rejects invalid inputs
// ---------------------------------------------------------------------------

/// An unknown `kind` value must produce a `ParseFailed` error, not silently
/// default to `None`.
#[test]
fn test_from_config_rejects_unknown_kind() {
    let err = TlsConfig::from_config(r#"kind = "jks""#).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("jks") || msg.contains("variant"),
        "unknown kind must produce a ParseFailed error; got: {msg}"
    );
}

/// An inline `password` field (not `password_env`) must be rejected by
/// `deny_unknown_fields` — inline passwords must never appear in config.
#[test]
fn test_from_config_rejects_inline_password_field() {
    let err = TlsConfig::from_config(
        r#"
            kind = "pkcs12"
            path = "/etc/client.p12"
            password = "secretplaintext"
        "#,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown") || msg.contains("password"),
        "inline password field must be rejected; got: {msg}"
    );
}

/// Missing `path` for `kind = "pem"` must produce a `ParseFailed` error.
#[test]
fn test_from_config_rejects_pem_without_path() {
    let err = TlsConfig::from_config(r#"kind = "pem""#).unwrap_err();
    assert!(
        matches!(err, Error::ParseFailed(_)),
        "pem without path must produce ParseFailed; got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// TlsConfig::swe_default
// ---------------------------------------------------------------------------

/// `swe_default()` must always return `TlsConfig::None`. If someone
/// accidentally changes the default config to PEM or PKCS12, every
/// consumer that calls `builder()` will start trying to load a cert file.
#[test]
fn test_swe_default_is_none() {
    let cfg = TlsConfig::swe_default().expect("swe_default must parse");
    assert!(
        matches!(cfg, TlsConfig::None),
        "swe_default must be TlsConfig::None; got: {cfg:?}"
    );
}

// ---------------------------------------------------------------------------
// Values flow through Builder unchanged
// ---------------------------------------------------------------------------

/// Each variant must survive the `Builder::with_config` round-trip without
/// modification — the builder must not normalise or switch variants.
#[test]
fn test_pem_variant_survives_builder_round_trip() {
    let b = Builder::with_config(TlsConfig::Pem {
        path: "/roundtrip/cert.pem".into(),
    });
    match b.config() {
        TlsConfig::Pem { path } => assert_eq!(path, "/roundtrip/cert.pem"),
        other => panic!("expected Pem, got: {other:?}"),
    }
}

#[test]
fn test_pkcs12_variant_survives_builder_round_trip() {
    let b = Builder::with_config(TlsConfig::Pkcs12 {
        path: "/roundtrip/cert.p12".into(),
        password_env: Some("ROUND_TRIP_ENV".into()),
    });
    match b.config() {
        TlsConfig::Pkcs12 { path, password_env } => {
            assert_eq!(path, "/roundtrip/cert.p12");
            assert_eq!(password_env.as_deref(), Some("ROUND_TRIP_ENV"));
        }
        other => panic!("expected Pkcs12, got: {other:?}"),
    }
}

#[test]
fn test_none_variant_survives_builder_round_trip() {
    let b = Builder::with_config(TlsConfig::None);
    assert!(matches!(b.config(), TlsConfig::None));
}
