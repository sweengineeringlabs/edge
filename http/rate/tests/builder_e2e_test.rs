//! End-to-end tests for builder/SAF facade.

#[test]
fn e2e_builder() {
    let _b = swe_http_rate::builder().unwrap();
}

#[test]
fn e2e_with_config_parses_custom_toml_and_flows_through_to_builder() {
    let toml = r#"
        tokens_per_second = 50
        burst_capacity = 100
        per_host = false
    "#;
    let cfg = swe_http_rate::RateConfig::from_config(toml)
        .expect("from_config parses");
    assert_eq!(cfg.tokens_per_second, 50);
    assert_eq!(cfg.burst_capacity, 100);
    assert!(!cfg.per_host);
    let b = swe_http_rate::Builder::with_config(cfg);
    assert_eq!(b.config().tokens_per_second, 50);
    let _layer = b.build().expect("build ok");
}

#[test]
fn e2e_config() {
    let b = swe_http_rate::builder().unwrap();
    let _cfg = b.config();
}

#[test]
fn e2e_build() {
    let b = swe_http_rate::builder().unwrap();
    let _layer = b.build().unwrap();
}

