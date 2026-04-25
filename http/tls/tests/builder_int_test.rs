//! Integration tests for `swe_http_tls` `Builder` and `builder()` SAF entry point.
//!
//! Covers: `builder()`, `Builder::with_config`, `Builder::config`, `Builder::build`.

use swe_http_tls::{builder, Builder, Error, TlsApplier, TlsConfig, TlsLayer};

// ---------------------------------------------------------------------------
// builder() — SAF entry point
// ---------------------------------------------------------------------------

/// The crate-shipped baseline TOML must always parse cleanly. If the file
/// is corrupted or missing, this is the first test to break.
#[test]
fn test_builder_fn_returns_ok_with_swe_default() {
    builder().expect("builder() must succeed with crate baseline");
}

/// The SWE default must be `TlsConfig::None` (pass-through) — tests and
/// services that don't configure mTLS must not accidentally load a cert.
#[test]
fn test_builder_fn_swe_default_is_tls_config_none() {
    let b = builder().expect("baseline parses");
    assert!(
        matches!(b.config(), TlsConfig::None),
        "swe_default config must be TlsConfig::None; got: {:?}",
        b.config()
    );
}

/// Building the `TlsConfig::None` baseline must succeed — a pass-through
/// layer must always be constructible.
#[test]
fn test_builder_fn_build_with_none_default_succeeds() {
    let _layer: TlsLayer = builder()
        .expect("baseline parses")
        .build()
        .expect("None config must always build");
}

// ---------------------------------------------------------------------------
// Builder::with_config — config variants
// ---------------------------------------------------------------------------

/// `TlsConfig::None` must build without error and produce a Debug string
/// containing "noop", confirming the pass-through provider was selected.
#[test]
fn test_with_config_none_builds_and_debug_contains_noop() {
    let layer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    let dbg = format!("{layer:?}");
    assert!(dbg.contains("noop"), "None config Debug must contain 'noop'; got: {dbg}");
}

/// `TlsConfig::Pem` with a missing file must fail fast at `build()` time,
/// not deferred to the first request. Missing-at-startup is preferable to
/// missing-at-runtime.
#[test]
fn test_with_config_pem_missing_file_fails_at_build_time() {
    let cfg = TlsConfig::Pem {
        path: "/this/path/definitely/does/not/exist.pem".into(),
    };
    let err = Builder::with_config(cfg).build().unwrap_err();
    assert!(
        matches!(err, Error::FileReadFailed { .. }),
        "missing PEM file must return FileReadFailed; got: {err:?}"
    );
}

/// `TlsConfig::Pkcs12` without a password and a missing file must fail fast.
#[test]
fn test_with_config_pkcs12_missing_file_fails_at_build_time() {
    let cfg = TlsConfig::Pkcs12 {
        path: "/this/path/definitely/does/not/exist.p12".into(),
        password_env: None,
    };
    let err = Builder::with_config(cfg).build().unwrap_err();
    assert!(
        matches!(err, Error::FileReadFailed { .. }),
        "missing PKCS12 file must return FileReadFailed; got: {err:?}"
    );
}

/// `TlsConfig::Pkcs12` with an unset password env var must return
/// `Error::MissingEnvVar` before even attempting to read the file.
#[test]
fn test_with_config_pkcs12_missing_password_env_returns_missing_env_var() {
    let env_name = "SWE_IT_TLS_BUILDER_PW_ABSENT_01";
    std::env::remove_var(env_name);
    let cfg = TlsConfig::Pkcs12 {
        path: "irrelevant.p12".into(),
        password_env: Some(env_name.into()),
    };
    let err = Builder::with_config(cfg).build().unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, env_name),
        other => panic!("expected MissingEnvVar, got: {other:?}"),
    }
}

/// `TlsConfig::None` must build successfully even when arbitrary environment
/// variables are set or unset — the None path must not read any env vars.
#[test]
fn test_with_config_none_ignores_environment() {
    std::env::set_var("SWE_IT_TLS_BUILDER_IGNORED_ENV", "some_value");
    Builder::with_config(TlsConfig::None)
        .build()
        .expect("None config must build regardless of env vars");
    std::env::remove_var("SWE_IT_TLS_BUILDER_IGNORED_ENV");
}

// ---------------------------------------------------------------------------
// Builder::config — accessor
// ---------------------------------------------------------------------------

/// `config()` must return the exact variant that was passed to `with_config`.
#[test]
fn test_config_accessor_returns_none_variant() {
    let b = Builder::with_config(TlsConfig::None);
    assert!(matches!(b.config(), TlsConfig::None));
}

/// `config()` must return the Pem variant with the exact path supplied.
#[test]
fn test_config_accessor_returns_pem_variant_with_correct_path() {
    let b = Builder::with_config(TlsConfig::Pem {
        path: "/some/cert.pem".into(),
    });
    match b.config() {
        TlsConfig::Pem { path } => assert_eq!(path, "/some/cert.pem"),
        other => panic!("expected Pem, got: {other:?}"),
    }
}

/// `config()` must return the Pkcs12 variant with the correct path and
/// password_env preserved.
#[test]
fn test_config_accessor_returns_pkcs12_variant_with_correct_fields() {
    let b = Builder::with_config(TlsConfig::Pkcs12 {
        path: "/some/cert.p12".into(),
        password_env: Some("SWE_IT_TLS_BUILDER_EXPECTED_ENV".into()),
    });
    match b.config() {
        TlsConfig::Pkcs12 { path, password_env } => {
            assert_eq!(path, "/some/cert.p12");
            assert_eq!(password_env.as_deref(), Some("SWE_IT_TLS_BUILDER_EXPECTED_ENV"));
        }
        other => panic!("expected Pkcs12, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// TlsLayer: Send + Sync
// ---------------------------------------------------------------------------

#[test]
fn test_tls_layer_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TlsLayer>();
}
