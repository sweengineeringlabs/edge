//! Integration tests exercising the public gateway surface of the swe_edge_http_cassette crate.

use swe_edge_http_cassette::{Builder, CassetteConfig, CassetteLayer, Error};

fn make_cfg(dir: &str) -> CassetteConfig {
    CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir.to_string(),
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec!["authorization".to_string()],
        scrub_body_paths: vec![],
    }
}

#[test]
fn test_builder_fn_loads_swe_default_and_succeeds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = make_cfg(&dir);
    Builder::with_config(cfg).build("test_builder_fn_default").expect("build must succeed");
}

#[test]
fn test_builder_fn_default_config_has_replay_mode() {
    let b = swe_edge_http_cassette::builder().expect("builder() must succeed");
    assert_eq!(b.config().mode, "replay", "swe_default mode must be replay");
}

#[test]
fn test_with_config_custom_config_stores_values() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = make_cfg(&dir);
    let b = Builder::with_config(cfg);
    assert_eq!(b.config().mode, "auto");
    assert!(b.config().match_on.contains(&"method".to_string()));
    assert!(b.config().scrub_headers.contains(&"authorization".to_string()));
}

#[test]
fn test_build_produces_cassette_layer() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let layer: CassetteLayer = Builder::with_config(make_cfg(&dir))
        .build("test_build_cassette_layer")
        .expect("build must succeed");
    let s = format!("{layer:?}");
    assert!(s.contains("CassetteLayer"), "Debug must contain 'CassetteLayer': {s}");
}

#[test]
fn test_cassette_layer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<CassetteLayer>();
}

#[test]
fn test_build_record_mode_succeeds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "record".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec![],
    };
    Builder::with_config(cfg).build("record_mode_test").expect("record mode must build");
}

#[test]
fn test_build_replay_mode_with_missing_file_succeeds() {
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
        .build("replay_missing_file")
        .expect("replay with missing file must build");
}

#[test]
fn test_build_with_empty_scrub_body_paths_succeeds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string()],
        scrub_headers: vec!["authorization".to_string()],
        scrub_body_paths: vec![],
    };
    Builder::with_config(cfg).build("empty_scrub_paths").expect("empty scrub paths must build");
}

#[test]
fn test_build_with_nested_scrub_body_paths_succeeds() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec!["metadata.trace_id".to_string(), "request_id".to_string()],
    };
    Builder::with_config(cfg).build("nested_scrub").expect("nested scrub paths must build");
}

#[test]
fn test_error_parse_failed_display_contains_crate_name() {
    let err = Error::ParseFailed("oops".to_string());
    let s = err.to_string();
    assert!(s.contains("swe_edge_http_cassette"), "ParseFailed Display must name the crate: {s}");
}

#[test]
fn test_error_not_implemented_display_is_non_empty() {
    let s = Error::NotImplemented("test").to_string();
    assert!(!s.is_empty(), "NotImplemented Display must not be empty");
}

#[test]
fn test_swe_default_scrub_headers_includes_authorization() {
    let b = swe_edge_http_cassette::builder().expect("builder() must succeed");
    let has_auth = b.config().scrub_headers.iter().any(|h| h.to_ascii_lowercase().contains("authorization"));
    assert!(has_auth, "swe_default scrub_headers must include authorization");
}
