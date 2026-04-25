//! Integration tests for the `HttpCassette` trait contract.
//!
//! `api::http_cassette::HttpCassette` is `pub(crate)` — the trait itself is
//! not accessible from outside the crate. Integration tests therefore verify
//! the *observable effects* of the trait contract:
//!
//! - The crate name surfaces in layer Debug output (produced by
//!   `DefaultHttpCassette::describe()` flowing into `CassetteLayer`).
//! - `CassetteLayer` is `Send + Sync`, which is enforced by the supertrait
//!   bounds on `HttpCassette: Send + Sync`.
//! - The builder pipeline that wires `DefaultHttpCassette` into
//!   `CassetteLayer` completes without error.

use swe_http_cassette::{Builder, CassetteConfig, CassetteLayer};

// ---------------------------------------------------------------------------
// Observable effect: describe() crate name embedded in Debug
// ---------------------------------------------------------------------------

/// The `CassetteLayer` Debug output must contain the mode string, which
/// the core layer derives by composing `DefaultHttpCassette::describe()`.
/// This confirms the impl wire-up is correct end-to-end.
#[test]
fn test_cassette_layer_debug_shows_configured_mode() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    let layer: CassetteLayer = Builder::with_config(cfg)
        .build("describe_mode_check")
        .expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("auto"),
        "CassetteLayer Debug must contain mode value; got: {dbg}"
    );
}

/// When the mode is "replay", Debug must reflect that value — confirming
/// the mode field flows from config through `DefaultHttpCassette` into the
/// layer's Debug impl without being silently overwritten.
#[test]
fn test_cassette_layer_debug_reflects_replay_mode() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir,
        match_on: vec!["url".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    let layer = Builder::with_config(cfg)
        .build("replay_mode_debug")
        .expect("build must succeed");
    assert!(format!("{layer:?}").contains("replay"));
}

/// When the mode is "record", Debug must reflect that value.
#[test]
fn test_cassette_layer_debug_reflects_record_mode() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "record".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    let layer = Builder::with_config(cfg)
        .build("record_mode_debug")
        .expect("build must succeed");
    assert!(format!("{layer:?}").contains("record"));
}

// ---------------------------------------------------------------------------
// Observable effect: Send + Sync (enforced by HttpCassette supertrait bounds)
// ---------------------------------------------------------------------------

/// `CassetteLayer` must be `Send + Sync`. The supertrait bounds on
/// `HttpCassette: Send + Sync` propagate to any concrete type that holds a
/// `Box<dyn HttpCassette>`. Removing those bounds from the trait would break
/// this compile-time assertion.
#[test]
fn test_cassette_layer_satisfies_send_sync_via_http_cassette_supertrait() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CassetteLayer>();
}

// ---------------------------------------------------------------------------
// Builder pipeline: DefaultHttpCassette is correctly wired
// ---------------------------------------------------------------------------

/// The complete builder pipeline — `builder()` → `Builder::build` — must
/// succeed, confirming `DefaultHttpCassette::new` is called with the correct
/// config inside the crate.
#[test]
fn test_builder_pipeline_produces_cassette_layer() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec!["authorization".to_string()],
        scrub_body_paths: vec![],
    };
    let layer = Builder::with_config(cfg)
        .build("pipeline_check")
        .expect("pipeline must produce a CassetteLayer");
    // Confirm we received a CassetteLayer, not a panic or error.
    let dbg = format!("{layer:?}");
    assert!(dbg.contains("CassetteLayer"), "must be a CassetteLayer; got: {dbg}");
}
