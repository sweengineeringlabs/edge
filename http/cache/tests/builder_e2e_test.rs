//! End-to-end tests for builder/SAF facade.

#[test]
fn e2e_builder() {
    let _b = swe_http_cache::builder().unwrap();
}

#[test]
fn e2e_with_config_parses_custom_toml_and_flows_through_to_builder() {
    let toml = r#"
        default_ttl_seconds = 60
        max_entries = 500
        respect_cache_control = false
        cache_private = true
    "#;
    let cfg = swe_http_cache::CacheConfig::from_config(toml)
        .expect("from_config parses");
    assert_eq!(cfg.default_ttl_seconds, 60);
    assert_eq!(cfg.max_entries, 500);
    assert!(!cfg.respect_cache_control);
    assert!(cfg.cache_private);
    let b = swe_http_cache::Builder::with_config(cfg);
    assert_eq!(b.config().default_ttl_seconds, 60);
    let _layer = b.build().expect("build ok");
}

#[test]
fn e2e_config() {
    let b = swe_http_cache::builder().unwrap();
    let _cfg = b.config();
}

#[test]
fn e2e_build() {
    let b = swe_http_cache::builder().unwrap();
    let _layer = b.build().unwrap();
}

