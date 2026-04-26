//! Integration tests for `swe_edge_egress_cassette` `Builder` and `builder()` SAF entry point.
//!
//! Covers: `builder()`, `Builder::with_config`, `Builder::config`, `Builder::build`.

use swe_edge_egress_cassette::{builder, Builder, CassetteConfig, CassetteLayer};

fn make_config(dir: &str) -> CassetteConfig {
    // Normalize backslashes so TOML doesn't treat `\U`, `\t`, etc. as escape
    // sequences inside the basic string.
    let dir_safe = dir.replace('\\', "/");
    CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir_safe,
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec!["authorization".to_string()],
        scrub_body_paths: vec![],
    }
}

// ---------------------------------------------------------------------------
// builder() — SAF entry point
// ---------------------------------------------------------------------------

/// The crate-shipped baseline TOML must always parse; otherwise no consumer
/// of this crate can bootstrap without supplying their own config.
#[test]
fn test_builder_fn_loads_swe_default_and_returns_ok() {
    builder().expect("builder() must succeed with crate baseline");
}

/// The SWE default mode is "replay" — tests must not accidentally record
/// real traffic when the caller forgets to override the mode.
#[test]
fn test_builder_fn_swe_default_mode_is_replay() {
    let b = builder().expect("baseline parses");
    assert_eq!(
        b.config().mode,
        "replay",
        "swe_default mode must be 'replay' to prevent accidental recording"
    );
}

/// `authorization` must be in the default scrub list so cassettes committed
/// to VCS cannot leak API credentials.
#[test]
fn test_builder_fn_swe_default_scrubs_authorization_header() {
    let b = builder().expect("baseline parses");
    let has_auth = b
        .config()
        .scrub_headers
        .iter()
        .any(|h| h.eq_ignore_ascii_case("authorization"));
    assert!(
        has_auth,
        "swe_default scrub_headers must include 'authorization'; got: {:?}",
        b.config().scrub_headers
    );
}

// ---------------------------------------------------------------------------
// Builder::with_config — custom config flows through unchanged
// ---------------------------------------------------------------------------

/// All fields supplied through `with_config` must be accessible via
/// `config()` without modification before `build` is called.
#[test]
fn test_with_config_stores_all_fields_unchanged() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = make_config(&dir);
    let b = Builder::with_config(cfg);

    assert_eq!(b.config().mode, "auto");
    assert_eq!(b.config().cassette_dir, dir);
    assert!(b.config().match_on.contains(&"method".to_string()));
    assert!(b.config().match_on.contains(&"url".to_string()));
    assert!(b.config().scrub_headers.contains(&"authorization".to_string()));
    assert!(b.config().scrub_body_paths.is_empty());
}

/// `config()` returns a reference to the policy that will be used at
/// runtime — the returned reference must reflect a custom multiplier change,
/// not some internal default.
#[test]
fn test_config_accessor_returns_stored_reference() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "record".to_string(),
        cassette_dir: dir,
        match_on: vec!["body_hash".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec!["request_id".to_string()],
    };
    let b = Builder::with_config(cfg);
    assert_eq!(b.config().mode, "record");
    assert_eq!(b.config().match_on, vec!["body_hash"]);
    assert_eq!(b.config().scrub_body_paths, vec!["request_id"]);
}

// ---------------------------------------------------------------------------
// Builder::build — produces a CassetteLayer
// ---------------------------------------------------------------------------

/// Happy path: `build` must succeed and return a `CassetteLayer` whose
/// Debug output identifies the type and records the configured mode.
#[test]
fn test_build_with_auto_mode_returns_cassette_layer() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let layer: CassetteLayer = Builder::with_config(make_config(&dir))
        .build("happy_path")
        .expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("CassetteLayer"),
        "Debug must contain 'CassetteLayer'; got: {dbg}"
    );
}

/// Building in "replay" mode with no pre-existing fixture file must succeed
/// — the layer starts with an empty in-memory map and only fails when a
/// request arrives with no recorded match.
#[test]
fn test_build_replay_mode_missing_fixture_file_succeeds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    Builder::with_config(cfg)
        .build("replay_no_fixture")
        .expect("replay with missing fixture must build");
}

/// Building in "record" mode must succeed so a fresh recording session can
/// start without requiring a pre-existing cassette file.
#[test]
fn test_build_record_mode_succeeds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "record".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    Builder::with_config(cfg)
        .build("record_session")
        .expect("record mode must build");
}

/// Multiple scrub body paths (including nested dot-paths) must not prevent
/// `build` from succeeding — path parsing happens lazily at request time.
#[test]
fn test_build_with_nested_scrub_body_paths_succeeds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string(), "body_hash".to_string()],
        scrub_headers: vec!["authorization".to_string()],
        scrub_body_paths: vec!["request_id".to_string(), "metadata.trace_id".to_string()],
    };
    Builder::with_config(cfg)
        .build("nested_scrub")
        .expect("nested scrub body paths must build");
}

// ---------------------------------------------------------------------------
// CassetteLayer: Send + Sync — compile-time proof
// ---------------------------------------------------------------------------

/// `CassetteLayer` must be `Send + Sync` so it can be used across async
/// task boundaries (e.g. shared via `Arc` in a `reqwest_middleware` chain).
#[test]
fn test_cassette_layer_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CassetteLayer>();
}
