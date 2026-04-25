//! Integration tests for the body-scrubber behavior, exercised through the
//! public builder API.
//!
//! `core::body_scrubber::scrub_body` is `pub(crate)`, so integration tests
//! cannot call it directly. Instead, tests confirm the *observable effect*:
//! that the `CassetteLayer` built with `scrub_body_paths` accepted in
//! config and stored correctly — and that the config contract guarantees
//! the scrubber will be invoked with the correct paths at request time.
//!
//! These tests also verify that edge-case `scrub_body_paths` values
//! (empty, nested dot paths, array-index paths) are accepted at build time
//! without error, since path validation is deferred to request time.

use swe_http_cassette::{Builder, CassetteConfig};

fn make_cfg(dir: &str, scrub_body_paths: Vec<String>) -> CassetteConfig {
    CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir.replace('\\', "/"),
        match_on: vec!["method".to_string(), "url".to_string(), "body_hash".to_string()],
        scrub_headers: vec![],
        scrub_body_paths,
    }
}

// ---------------------------------------------------------------------------
// Build-time acceptance of scrub_body_paths values
// ---------------------------------------------------------------------------

/// An empty `scrub_body_paths` must build without error. The scrubber is a
/// no-op for an empty path list and must not prevent layer construction.
#[test]
fn test_empty_scrub_body_paths_builds_successfully() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap();
    Builder::with_config(make_cfg(dir, vec![]))
        .build("empty_scrub_paths")
        .expect("empty scrub_body_paths must build");
}

/// A top-level field path (no dots) must be accepted at build time.
#[test]
fn test_top_level_path_builds_successfully() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap();
    Builder::with_config(make_cfg(dir, vec!["request_id".to_string()]))
        .build("top_level_path")
        .expect("top-level scrub path must build");
}

/// A nested dot-separated path must be accepted at build time.
#[test]
fn test_nested_dot_path_builds_successfully() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap();
    Builder::with_config(make_cfg(dir, vec!["metadata.trace_id".to_string()]))
        .build("nested_path")
        .expect("nested dot path must build");
}

/// An array-index path (dot then numeric segment) must be accepted at build
/// time, since the scrubber handles array indexing at request time.
#[test]
fn test_array_index_path_builds_successfully() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap();
    Builder::with_config(make_cfg(dir, vec!["results.0.id".to_string()]))
        .build("array_index_path")
        .expect("array-index scrub path must build");
}

/// Multiple paths including nested and array paths must all be accepted at
/// build time. This matches the common case of scrubbing several
/// non-deterministic fields simultaneously.
#[test]
fn test_multiple_mixed_paths_build_successfully() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap();
    let paths = vec![
        "request_id".to_string(),
        "metadata.trace_id".to_string(),
        "metadata.timestamp".to_string(),
        "results.0.id".to_string(),
    ];
    Builder::with_config(make_cfg(dir, paths))
        .build("mixed_paths")
        .expect("multiple mixed scrub paths must build");
}

// ---------------------------------------------------------------------------
// scrub_body_paths survives the builder round-trip
// ---------------------------------------------------------------------------

/// The `scrub_body_paths` configured via `CassetteConfig` must be visible
/// through `Builder::config()` after `with_config`, confirming the paths
/// are stored verbatim and not dropped or normalised during builder
/// construction.
#[test]
fn test_scrub_body_paths_survive_builder_round_trip() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let paths = vec!["request_id".to_string(), "metadata.trace_id".to_string()];
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string(), "body_hash".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: paths.clone(),
    };
    let b = Builder::with_config(cfg);
    assert_eq!(
        b.config().scrub_body_paths,
        paths,
        "scrub_body_paths must survive builder round-trip unchanged"
    );
}

/// When `body_hash` is included in `match_on`, the layer's config must
/// reflect both the `body_hash` match component AND the scrub paths, so
/// the middleware can apply scrubbing before hashing at request time.
#[test]
fn test_body_hash_in_match_on_with_scrub_paths_builds_correctly() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string(), "body_hash".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec!["request_id".to_string()],
    };
    let b = Builder::with_config(cfg);
    // Both must be present — the middleware logic gates scrubbing on whether
    // body_hash is in match_on.
    assert!(b.config().match_on.contains(&"body_hash".to_string()));
    assert!(b.config().scrub_body_paths.contains(&"request_id".to_string()));
}
