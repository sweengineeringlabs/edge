//! End-to-end tests for builder/SAF facade.

#[test]
fn e2e_builder() {
    let _b = swe_http_cassette::builder().unwrap();
}

#[test]
fn e2e_with_config_parses_custom_toml_and_flows_through_to_builder() {
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let dir = tmpdir.path().to_str().unwrap().replace('\\', "/");
    let toml = format!(
        r#"
            mode = "auto"
            cassette_dir = "{dir}"
            match_on = ["method", "url"]
            scrub_headers = ["authorization"]
            scrub_body_paths = []
        "#
    );
    let cfg = swe_http_cassette::CassetteConfig::from_config(&toml)
        .expect("from_config parses");
    assert_eq!(cfg.mode, "auto");
    assert_eq!(cfg.scrub_headers, vec!["authorization".to_string()]);
    let b = swe_http_cassette::Builder::with_config(cfg);
    assert_eq!(b.config().mode, "auto");
    let _layer = b.build("fresh_case").expect("build ok");
}

#[test]
fn e2e_config() {
    let b = swe_http_cassette::builder().unwrap();
    let _cfg = b.config();
}

#[test]
fn e2e_build() {
    let b = swe_http_cassette::builder().unwrap();
    let _layer = b.build("test_case").unwrap();
}

