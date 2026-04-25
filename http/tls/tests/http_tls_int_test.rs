//! Integration tests for the `HttpTls` trait contract.
//!
//! `api::http_tls::HttpTls` is `pub(crate)`. Integration tests verify the
//! observable effects of the trait through the public builder pipeline:
//!
//! - `TlsLayer::apply_to` succeeds for the `None` (pass-through) variant.
//! - `TlsLayer` is `Send + Sync` (enforced by `HttpTls: Send + Sync`).
//! - The layer produced by the builder wraps a provider whose `describe()`
//!   value appears in the Debug output.
//! - Missing files / env vars surface as typed errors, not panics.

use swe_http_tls::{Builder, Error, TlsApplier, TlsConfig, TlsLayer};

// ---------------------------------------------------------------------------
// HttpTls::identity — None provider returns Ok(None)
// ---------------------------------------------------------------------------

/// For `TlsConfig::None`, `apply_to` must leave the `ClientBuilder`
/// unchanged and return `Ok`. This is the base contract: pass-through
/// must never fail.
#[test]
fn test_apply_to_with_none_config_returns_ok() {
    let layer: TlsLayer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    layer
        .apply_to(reqwest::Client::builder())
        .expect("None apply_to must return Ok");
}

/// `apply_to` can be called multiple times on the same layer without error —
/// the identity is resolved once at build time and the layer is reusable.
#[test]
fn test_apply_to_none_is_reusable() {
    let layer: TlsLayer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    layer
        .apply_to(reqwest::Client::builder())
        .expect("first apply_to");
    layer
        .apply_to(reqwest::Client::builder())
        .expect("second apply_to must also succeed");
}

// ---------------------------------------------------------------------------
// HttpTls::describe — "noop" appears in Debug for TlsConfig::None
// ---------------------------------------------------------------------------

/// The Debug output of a `TlsConfig::None` layer must contain "noop",
/// which is the value `NoopHttpTls::describe()` returns.
#[test]
fn test_none_layer_debug_contains_noop_describe_value() {
    let layer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("noop"),
        "TlsConfig::None Debug must contain 'noop'; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// HttpTls: identity() errors for invalid cert data
// ---------------------------------------------------------------------------

/// `TlsConfig::Pem` with a missing file must surface as
/// `Error::FileReadFailed` at build time (because `PemHttpTls::load`
/// reads the file eagerly).
#[test]
fn test_pem_missing_file_returns_file_read_failed_at_build_time() {
    let cfg = TlsConfig::Pem {
        path: "/does/not/exist/cert.pem".into(),
    };
    let err = Builder::with_config(cfg).build().unwrap_err();
    assert!(
        matches!(err, Error::FileReadFailed { .. }),
        "missing PEM must return FileReadFailed; got: {err:?}"
    );
}

/// `TlsConfig::Pkcs12` with a missing file must surface as
/// `Error::FileReadFailed` at build time.
#[test]
fn test_pkcs12_missing_file_returns_file_read_failed_at_build_time() {
    let cfg = TlsConfig::Pkcs12 {
        path: "/does/not/exist/cert.p12".into(),
        password_env: None,
    };
    let err = Builder::with_config(cfg).build().unwrap_err();
    assert!(
        matches!(err, Error::FileReadFailed { .. }),
        "missing PKCS12 file must return FileReadFailed; got: {err:?}"
    );
}

/// `TlsConfig::Pkcs12` with a missing password env var must surface as
/// `Error::MissingEnvVar` — before even attempting to read the file.
#[test]
fn test_pkcs12_missing_password_env_returns_missing_env_var() {
    let env_name = "SWE_IT_HTTP_TLS_PW_ABSENT_02";
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

// ---------------------------------------------------------------------------
// TlsLayer: Send + Sync (enforced by HttpTls supertrait bounds)
// ---------------------------------------------------------------------------

/// `TlsLayer` must be `Send + Sync`. The `HttpTls: Send + Sync + Debug`
/// supertraits propagate this requirement through the `Arc<dyn HttpTls>`
/// held by `TlsLayer`.
#[test]
fn test_tls_layer_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TlsLayer>();
}

/// `Arc<TlsLayer>` must be constructible and `Send + Sync`.
#[test]
fn test_tls_layer_is_arc_send_sync() {
    use std::sync::Arc;
    let layer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    let _arc: Arc<TlsLayer> = Arc::new(layer);
}
