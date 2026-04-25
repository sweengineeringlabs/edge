//! End-to-end tests for the swe_edge_http_cassette SAF builder surface.

use swe_edge_http_cassette::{Builder, CassetteConfig, CassetteLayer};

fn make_cfg(dir: &str) -> CassetteConfig {
    CassetteConfig {
        mode: "auto".to_string(),
        cassette_dir: dir.to_string(),
        match_on: vec!["method".to_string(), "url".to_string()],
        scrub_headers: vec!["authorization".to_string()],
        scrub_body_paths: vec![],
    }
}

/// @covers: builder
#[test]
fn e2e_builder() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let b = swe_edge_http_cassette::builder().expect("builder() must succeed");
    assert_eq!(b.config().mode, "replay");
    let _layer: CassetteLayer = Builder::with_config(make_cfg(&dir))
        .build("e2e_builder_test")
        .expect("build must succeed");
}

/// @covers: Builder::with_config
#[test]
fn e2e_with_config() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let b = Builder::with_config(make_cfg(&dir));
    assert_eq!(b.config().mode, "auto");
    b.build("e2e_with_config_test").expect("build must succeed");
}

/// @covers: Builder::config
#[test]
fn e2e_config() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let b = Builder::with_config(make_cfg(&dir));
    assert!(b.config().match_on.contains(&"url".to_string()));
    assert!(b.config().scrub_headers.contains(&"authorization".to_string()));
}

/// @covers: Builder::build
#[test]
fn e2e_build() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let cfg = CassetteConfig {
        mode: "record".to_string(),
        cassette_dir: dir,
        match_on: vec!["method".to_string()],
        scrub_headers: vec![],
        scrub_body_paths: vec!["meta.id".to_string()],
    };
    let layer = Builder::with_config(cfg).build("e2e_build_test").expect("e2e build must succeed");
    assert!(!format!("{layer:?}").is_empty());
}
