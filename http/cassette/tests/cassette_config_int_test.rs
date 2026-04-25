//! Integration tests for `CassetteConfig` public surface.
//!
//! `CassetteConfig` is a plain struct with public fields — tests exercise
//! that struct literal construction round-trips correctly through the
//! `Builder` and that field values are preserved without mutation.

use swe_edge_http_cassette::{Builder, CassetteConfig};

// ---------------------------------------------------------------------------
// Struct construction — all public fields must be writable
// ---------------------------------------------------------------------------

/// Every field on `CassetteConfig` must be directly settable via struct
/// literal syntax. If a field is renamed or removed this test fails to
/// compile, catching the API break immediately.
#[test]
fn test_cassette_config_struct_fields_are_all_public() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir.clone(),
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec!["authorization".to_string()],
        scrub_body_paths: vec!["request_id".to_string()],
    };
    // Each field must read back what we wrote.
    assert_eq!(cfg.mode, "auto");
    assert_eq!(cfg.cassette_dir, dir);
    assert_eq!(cfg.match_on, vec!["method", "url"]);
    assert_eq!(cfg.scrub_headers, vec!["authorization"]);
    assert_eq!(cfg.scrub_body_paths, vec!["request_id"]);
}

/// `CassetteConfig` must be `Clone` so it can be threaded through the
/// builder pipeline without requiring the caller to reconstruct it.
#[test]
fn test_cassette_config_is_clone() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir,
        match_on: vec!["url".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    let cloned = cfg.clone();
    assert_eq!(cloned.mode, cfg.mode);
    assert_eq!(cloned.cassette_dir, cfg.cassette_dir);
}

// ---------------------------------------------------------------------------
// mode — valid mode strings flow through to the layer
// ---------------------------------------------------------------------------

/// "replay" mode must build a layer without error. This is the default
/// mode and must always be a legal value.
#[test]
fn test_mode_replay_is_accepted_by_build() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    Builder::with_config(cfg).build("mode_replay").expect("replay mode must build");
}

/// "record" mode must build a layer without error.
#[test]
fn test_mode_record_is_accepted_by_build() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "record".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    Builder::with_config(cfg).build("mode_record").expect("record mode must build");
}

/// "auto" mode must build a layer without error.
#[test]
fn test_mode_auto_is_accepted_by_build() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    Builder::with_config(cfg).build("mode_auto").expect("auto mode must build");
}

// ---------------------------------------------------------------------------
// match_on — all documented match components must be accepted
// ---------------------------------------------------------------------------

/// `match_on` including all three documented components must be accepted
/// at build time. Parsing of unknown components is deferred to request
/// time (they are silently ignored), but the three standard ones are
/// functional and must never cause a build failure.
#[test]
fn test_match_on_with_all_standard_components_builds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string(), "body_hash".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    Builder::with_config(cfg).build("match_on_all").expect("all match_on components must build");
}

/// An empty `match_on` is a degenerate but valid config — every request
/// maps to the same empty key. This must build without error.
#[test]
fn test_match_on_empty_builds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir,
        match_on: vec![],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    Builder::with_config(cfg).build("match_on_empty").expect("empty match_on must build");
}

// ---------------------------------------------------------------------------
// scrub_headers — security: authorization must survive round-trip
// ---------------------------------------------------------------------------

/// `scrub_headers` containing "authorization" must survive the round-trip
/// through `Builder::with_config` into the layer's config, confirming the
/// scrub list is not silently cleared during build.
#[test]
fn test_scrub_headers_survives_build_round_trip() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec!["authorization".to_string(), "set-cookie".to_string()],
        scrub_body_paths: vec![],
    };
    let b = Builder::with_config(cfg);
    assert!(b.config().scrub_headers.contains(&"authorization".to_string()));
    assert!(b.config().scrub_headers.contains(&"set-cookie".to_string()));
}

// ---------------------------------------------------------------------------
// scrub_body_paths — dot-notation paths must be stored verbatim
// ---------------------------------------------------------------------------

/// Dot-separated body paths must be stored exactly as supplied, since the
/// scrubber splits on `.` at request time. Premature splitting would break
/// nested field removal.
#[test]
fn test_scrub_body_paths_stored_verbatim() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![
            "request_id".to_string(),
            "metadata.trace_id".to_string(),
            "results.0.id".to_string(),
        ],
    };
    assert_eq!(cfg.scrub_body_paths[0], "request_id");
    assert_eq!(cfg.scrub_body_paths[1], "metadata.trace_id");
    assert_eq!(cfg.scrub_body_paths[2], "results.0.id");
}
