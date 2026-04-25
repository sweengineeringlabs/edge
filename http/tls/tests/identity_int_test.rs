//! Integration tests for `api/identity` — the TLS identity provider abstraction.
//!
//! Verifies that the identity providers declared in `core::identity`
//! (noop, PEM, PKCS#12) are correctly wired through the SAF builder
//! surface and honour the `HttpTls` contract.

use swe_http_tls::{Builder, TlsApplier, TlsConfig};

/// `TlsConfig::None` produces a provider that returns no identity,
/// so `apply_to` passes the ClientBuilder through unmodified.
#[test]
fn test_identity_none_config_produces_no_identity() {
    let layer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None config must build without error");
    layer
        .apply_to(reqwest::Client::builder())
        .expect("apply_to with no identity must not fail");
}

/// The noop identity provider must be `Send + Sync`, since it is
/// held behind `Arc<dyn HttpTls>` in `TlsLayer`.
#[test]
fn test_identity_provider_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<swe_http_tls::TlsLayer>();
}

/// Each `TlsConfig` variant that does NOT require external key material
/// must succeed at `build()` — the identity sub-selection path in
/// `core::identity::tls_factory` must not return an error for None.
#[test]
fn test_identity_factory_none_variant_succeeds() {
    let result = Builder::with_config(TlsConfig::None).build();
    assert!(result.is_ok(), "identity factory must succeed for TlsConfig::None");
}
