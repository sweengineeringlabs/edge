//! Integration tests for `swe_edge_http_tls` trait re-exports (`api/traits.rs`).
//!
//! `api/traits.rs` defines the `pub(crate)` type alias `HttpTlsTrait` for
//! `dyn HttpTls`. The integration-level contract is:
//!
//! - The SAF re-export surface is complete: `TlsConfig`, `TlsLayer`,
//!   `Builder`, `Error`, and `builder()` are all accessible.
//! - `TlsLayer::apply_to` works end-to-end with a `reqwest::ClientBuilder`.
//! - `TlsLayer` is `Send + Sync` (flows from `HttpTls: Send + Sync + Debug`).

use swe_edge_http_tls::{builder, Builder, Error, TlsApplier, TlsConfig, TlsLayer};

// ---------------------------------------------------------------------------
// SAF re-export completeness — compile-time proof
// ---------------------------------------------------------------------------

/// All five public items exported by the SAF surface must be reachable from
/// the crate root. A missing re-export causes this test to fail to compile.
#[test]
fn test_saf_surface_exports_all_required_types() {
    // builder() — function
    let _ = builder as fn() -> Result<swe_edge_http_tls::Builder, Error>;

    // Builder — type
    fn accept_builder(_: Builder) {}
    let _ = accept_builder as fn(Builder);

    // TlsConfig — type
    let _ = TlsConfig::None;

    // TlsLayer — type
    fn accept_layer(_: TlsLayer) {}
    let _ = accept_layer as fn(TlsLayer);

    // Error — type
    let _e = Error::NotImplemented("test");
}

// ---------------------------------------------------------------------------
// HttpTls trait object safety — Arc<dyn HttpTls> must compile
// ---------------------------------------------------------------------------

/// `HttpTls` must be object-safe (Send + Sync + Debug supertrait bounds
/// are satisfied). Although `HttpTls` itself is `pub(crate)`, its effect
/// propagates to `TlsLayer` which holds `Arc<dyn HttpTls>`. If the trait
/// loses object-safety this test fails to compile.
#[test]
fn test_tls_layer_holds_arc_dyn_provider() {
    // The TlsLayer itself proves Arc<dyn HttpTls> works.
    let layer: TlsLayer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    // If Arc<dyn HttpTls> weren't working, build() would fail to compile.
    drop(layer);
}

// ---------------------------------------------------------------------------
// Send + Sync — required by HttpTls supertrait bounds
// ---------------------------------------------------------------------------

#[test]
fn test_tls_layer_satisfies_send_sync_from_http_tls_supertraits() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TlsLayer>();
}

// ---------------------------------------------------------------------------
// End-to-end pipeline: builder → layer → apply_to → client
// ---------------------------------------------------------------------------

/// The full pipeline through the SAF surface must compile and run without
/// panic for the `TlsConfig::None` case.
#[test]
fn test_full_saf_pipeline_none_config_builds_client() {
    let layer: TlsLayer = builder()
        .expect("builder() must succeed")
        .build()
        .expect("None config must build");
    let cb = layer
        .apply_to(reqwest::Client::builder())
        .expect("apply_to must succeed");
    let _client = cb.build().expect("ClientBuilder must build");
}

/// The full pipeline through `Builder::with_config(TlsConfig::None)` must
/// also produce a working client.
#[test]
fn test_full_with_config_pipeline_none_config_builds_client() {
    let layer = Builder::with_config(TlsConfig::None)
        .build()
        .expect("None must build");
    let _client = layer
        .apply_to(reqwest::Client::builder())
        .expect("apply_to")
        .build()
        .expect("build client");
}
