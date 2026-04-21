//! End-to-end tests for builder/SAF facade.

#[test]
fn e2e_builder() {
    let _b = swe_http_tls::builder().unwrap();
}

#[test]
fn e2e_with_config_parses_custom_toml_and_builds_layer() {
    let toml = r#"kind = "none""#;
    let cfg = swe_http_tls::TlsConfig::from_config(toml)
        .expect("from_config parses");
    assert!(matches!(cfg, swe_http_tls::TlsConfig::None));
    let b = swe_http_tls::Builder::with_config(cfg);
    let _layer = b.build().expect("build ok");
}

#[test]
fn e2e_config() {
    let b = swe_http_tls::builder().unwrap();
    let _cfg = b.config();
}

#[test]
fn e2e_build() {
    let b = swe_http_tls::builder().unwrap();
    let _layer = b.build().unwrap();
}

