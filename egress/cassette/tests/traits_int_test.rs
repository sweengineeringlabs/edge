//! Integration tests for `swe_edge_egress_cassette` trait re-exports (`api/traits.rs`).
//!
//! `api/traits.rs` declares a `pub(crate)` type alias `HttpCassetteTrait`
//! for `dyn HttpCassette`. That alias is internal-only. From the integration
//! surface the relevant contract is: the SAF re-export surface is complete
//! and the `CassetteLayer` produced by the builder satisfies all trait bounds
//! required for use inside `reqwest_middleware::ClientBuilder`.
//!
//! These tests confirm that the full middleware trait chain compiles — if
//! `reqwest_middleware::Middleware` is removed from `CassetteLayer`'s impl
//! set, or if the trait object indirection through `HttpCassetteTrait` is
//! broken, consumers of the crate would fail to compile.

use swe_edge_egress_cassette::{Builder, CassetteConfig, CassetteLayer};

// ---------------------------------------------------------------------------
// reqwest_middleware::Middleware — CassetteLayer must implement it
// ---------------------------------------------------------------------------

/// `CassetteLayer` must implement `reqwest_middleware::Middleware` so it can
/// be attached to a `reqwest_middleware::ClientBuilder` via `.with(layer)`.
/// If the impl is missing this compile-time assertion fails.
#[test]
fn test_cassette_layer_implements_reqwest_middleware() {
    fn assert_middleware<T: reqwest_middleware::Middleware>() {}
    assert_middleware::<CassetteLayer>();
}

// ---------------------------------------------------------------------------
// CassetteLayer is usable in an Arc<T> context (trait-object indirection)
// ---------------------------------------------------------------------------

/// `CassetteLayer` must be `Send + Sync` so it can be wrapped in `Arc<T>`
/// and shared across async executor threads, as required by
/// `reqwest_middleware::ClientBuilder::with(Arc<impl Middleware>)`.
#[test]
fn test_cassette_layer_is_arc_send_sync() {
    use std::sync::Arc;
    fn assert_arc_safe<T: Send + Sync + 'static>() {
        let _ = std::mem::size_of::<Arc<T>>();
    }
    assert_arc_safe::<CassetteLayer>();
}

// ---------------------------------------------------------------------------
// Full pipeline: layer can be passed to reqwest_middleware::ClientBuilder
// ---------------------------------------------------------------------------

/// Building a `reqwest_middleware::ClientWithMiddleware` with a
/// `CassetteLayer` must compile and construct without panic. This test
/// does not make a real HTTP request; it validates that the type chain
/// from `CassetteLayer` → `reqwest_middleware::Middleware` → `ClientBuilder`
/// is fully wired.
#[test]
fn test_cassette_layer_attaches_to_reqwest_middleware_client_builder() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    let layer = Builder::with_config(cfg)
        .build("trait_chain_check")
        .expect("build must succeed");

    let _client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
        .with(layer)
        .build();
    // No assertion on `_client`'s behavior — the test proves the type chain
    // compiles and the constructor succeeds.
}
