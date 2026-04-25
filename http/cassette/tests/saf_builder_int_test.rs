//! Integration tests for `saf::builder` — the SAF-layer builder module.
//!
//! `saf/builder.rs` contains:
//! - `builder()` — the public SAF entry point, loads SWE baseline config.
//! - `Builder::with_config` — accepts a caller-supplied `CassetteConfig`.
//! - `Builder::config` — accessor.
//! - `Builder::build` — finalises into a `CassetteLayer`.
//!
//! The `saf/builder.rs` module also re-exports `api::builder::Builder`,
//! making it the single authoritative builder type through the public API.

use swe_edge_http_cassette::{builder, Builder, CassetteConfig, CassetteLayer, Error};

// ---------------------------------------------------------------------------
// builder() — SAF entry point: always returns Ok with default config
// ---------------------------------------------------------------------------

/// `builder()` must succeed unconditionally. If the crate-shipped
/// `config/application.toml` ever breaks, this is the first test to fail.
#[test]
fn test_saf_builder_fn_returns_ok() {
    let result = builder();
    assert!(result.is_ok(), "builder() must return Ok; got: {result:?}");
}

/// The SWE default mode must be "replay". Any change to the default TOML
/// that switches mode to "record" would cause integration tests to make
/// real network calls — catching that here prevents accidental recordings.
#[test]
fn test_saf_builder_fn_default_mode_is_replay() {
    let b = builder().unwrap();
    assert_eq!(
        b.config().mode,
        "replay",
        "builder() default mode must be 'replay'; got '{}'",
        b.config().mode
    );
}

/// The SWE default match_on list must not be empty. An empty list would
/// produce a single match key for every request, making all requests
/// collapse onto the same fixture.
#[test]
fn test_saf_builder_fn_default_match_on_is_not_empty() {
    let b = builder().unwrap();
    assert!(
        !b.config().match_on.is_empty(),
        "builder() default match_on must not be empty"
    );
}

// ---------------------------------------------------------------------------
// Builder::with_config + build — custom config flow
// ---------------------------------------------------------------------------

/// Custom config supplied via `with_config` must be accepted and produce a
/// `CassetteLayer` without error. This proves the SAF builder layer passes
/// the config through to `CassetteLayer::new` unchanged.
#[test]
fn test_saf_with_config_and_build_produces_cassette_layer() {
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
        .build("saf_with_config")
        .expect("with_config + build must succeed");
    let dbg = format!("{layer:?}");
    assert!(dbg.contains("CassetteLayer"), "must be a CassetteLayer; got: {dbg}");
}

/// `build` takes a cassette name which becomes part of the on-disk path.
/// Two calls with different names must produce distinct cassette files.
#[test]
fn test_saf_build_uses_cassette_name_in_path() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg_a = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir.clone(),
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    let cfg_b = CassetteConfig {
        mode: "replay".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    let l_a = Builder::with_config(cfg_a).build("cassette_alpha").unwrap();
    let l_b = Builder::with_config(cfg_b).build("cassette_beta").unwrap();
    let dbg_a = format!("{l_a:?}");
    let dbg_b = format!("{l_b:?}");
    // The cassette path is embedded in the Debug output; it must differ.
    assert_ne!(
        dbg_a, dbg_b,
        "different cassette names must produce different Debug output"
    );
}

// ---------------------------------------------------------------------------
// Error propagation through the SAF builder
// ---------------------------------------------------------------------------

/// `ParseFailed` constructed directly must surface through `Error::ParseFailed`
/// and embed the crate name in its display — confirming the error type
/// exported by the SAF surface is the same one `builder()` would return on
/// a malformed config.
#[test]
fn test_saf_error_parse_failed_display_names_crate() {
    let err = Error::ParseFailed("bad field".to_string());
    assert!(
        err.to_string().contains("swe_edge_http_cassette"),
        "Error::ParseFailed from SAF layer must name the crate"
    );
}
