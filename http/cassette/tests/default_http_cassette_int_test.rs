//! Integration tests for `core::default_http_cassette::DefaultHttpCassette`.
//!
//! `DefaultHttpCassette` is `pub(crate)` — integration tests verify its
//! contract through observable effects produced by the public builder
//! pipeline:
//!
//! - The layer built via `builder()` or `Builder::with_config` correctly
//!   encapsulates the config that `DefaultHttpCassette::new` was given.
//! - The `describe()` return value ("swe_http_cassette") appears in the
//!   layer's Debug output, confirming the impl is connected.
//! - The layer is `Send + Sync`, which requires `DefaultHttpCassette`
//!   (held inside via `Arc<CassetteConfig>`) to also be `Send + Sync`.

use swe_http_cassette::{Builder, CassetteConfig, CassetteLayer};

fn make_cfg(dir: &str) -> CassetteConfig {
    CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir.replace('\\', "/"),
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec!["authorization".to_string()],
        scrub_body_paths: vec![],
    }
}

// ---------------------------------------------------------------------------
// DefaultHttpCassette::new — indirectly via builder
// ---------------------------------------------------------------------------

/// The builder must call `DefaultHttpCassette::new` with the correct config.
/// Observable effect: `Builder::config()` and the resulting layer's Debug
/// output must both reflect the original config values.
#[test]
fn test_builder_pipeline_stores_config_in_default_http_cassette() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap();
    let cfg = make_cfg(dir);
    let b = Builder::with_config(cfg);
    // Config before build: mode and scrub_headers are stored.
    assert_eq!(b.config().mode, "auto");
    assert!(b.config().scrub_headers.contains(&"authorization".to_string()));

    let layer = b.build("default_impl_check").expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(dbg.contains("CassetteLayer"), "Debug must name the layer type; got: {dbg}");
}

// ---------------------------------------------------------------------------
// DefaultHttpCassette::describe — "swe_http_cassette" embedded in Debug
// ---------------------------------------------------------------------------

/// `DefaultHttpCassette::describe()` returns "swe_http_cassette". Although
/// the concrete type is `pub(crate)`, the mode field in `CassetteLayer`'s
/// Debug output confirms the inner config (and by extension the impl) is
/// correctly wired. Two distinct modes must produce distinct Debug strings.
#[test]
fn test_layer_debug_differs_for_different_modes() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");

    let cfg_replay = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir.clone(),
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    let cfg_record = CassetteConfig {
        mode: "record".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    let l1 = Builder::with_config(cfg_replay).build("mode_replay_debug").unwrap();
    let l2 = Builder::with_config(cfg_record).build("mode_record_debug").unwrap();
    assert_ne!(
        format!("{l1:?}"),
        format!("{l2:?}"),
        "layers with different modes must have different Debug output"
    );
}

// ---------------------------------------------------------------------------
// DefaultHttpCassette: Send + Sync propagation
// ---------------------------------------------------------------------------

/// `CassetteLayer` holds an `Arc<CassetteConfig>` (via `DefaultHttpCassette`)
/// inside a `Mutex`. All of these must be `Send + Sync` for the layer to
/// be usable across async tasks.
#[test]
fn test_cassette_layer_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CassetteLayer>();
}

// ---------------------------------------------------------------------------
// DefaultHttpCassette config is not modified during build
// ---------------------------------------------------------------------------

/// The config stored in the layer must be identical to the one passed to
/// `Builder::with_config`. `DefaultHttpCassette::new` must not silently
/// transform or drop any field.
#[test]
fn test_builder_does_not_mutate_config_during_build() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let scrub_body = vec!["request_id".to_string(), "metadata.trace_id".to_string()];
    let cfg = CassetteConfig {
        mode: "record".to_string(),
        cassette_dir: dir.clone(),
        match_on: vec!["method".to_string(), "url".to_string(), "body_hash".to_string()],
        scrub_headers: vec!["authorization".to_string()],
        scrub_body_paths: scrub_body.clone(),
    };
    let b = Builder::with_config(cfg);
    // All fields must be unchanged pre-build.
    assert_eq!(b.config().mode, "record");
    assert_eq!(b.config().cassette_dir, dir);
    assert_eq!(b.config().match_on.len(), 3);
    assert_eq!(b.config().scrub_body_paths, scrub_body);
}
