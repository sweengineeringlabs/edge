//! End-to-end tests for the swe_edge_http_tls SAF builder surface.

use swe_edge_http_tls::{Builder, TlsApplier, TlsConfig, TlsLayer};

/// @covers: builder
#[test]
fn e2e_builder() {
    let layer: TlsLayer = swe_edge_http_tls::builder()
        .expect("builder() must succeed")
        .build()
        .expect("build() must succeed");
    assert!(format!("{layer:?}").contains("noop"), "e2e: None config must produce noop layer");
}

/// @covers: Builder::with_config
#[test]
fn e2e_with_config() {
    let b = Builder::with_config(TlsConfig::None);
    assert!(matches!(b.config(), TlsConfig::None));
    b.build().expect("e2e with_config None build must succeed");
}

/// @covers: Builder::config
#[test]
fn e2e_config() {
    let cfg = TlsConfig::Pem { path: "/some/cert.pem".into() };
    let b = Builder::with_config(cfg);
    assert!(matches!(b.config(), TlsConfig::Pem { .. }), "config() must return Pem variant");
}

/// @covers: Builder::build
#[test]
fn e2e_build() {
    let layer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("e2e build with None must always succeed");
    assert!(!format!("{layer:?}").is_empty());
}
