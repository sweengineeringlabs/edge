//! End-to-end tests for builder/SAF facade.

#[test]
fn e2e_builder() {
    let _b = swe_http_retry::builder().unwrap();
}

#[test]
fn e2e_with_config_parses_custom_toml_and_flows_through_to_builder() {
    let toml = r#"
        max_retries = 5
        initial_interval_ms = 100
        max_interval_ms = 5000
        multiplier = 3.0
        retryable_statuses = [429, 503]
        retryable_methods = ["GET"]
    "#;
    let cfg = swe_http_retry::RetryConfig::from_config(toml)
        .expect("from_config parses");
    assert_eq!(cfg.max_retries, 5);
    assert_eq!(cfg.initial_interval_ms, 100);
    assert_eq!(cfg.multiplier, 3.0);
    let b = swe_http_retry::Builder::with_config(cfg);
    assert_eq!(b.config().max_retries, 5);
    let _layer = b.build().expect("build ok");
}

#[test]
fn e2e_config() {
    let b = swe_http_retry::builder().unwrap();
    let _cfg = b.config();
}

#[test]
fn e2e_build() {
    let b = swe_http_retry::builder().unwrap();
    let _layer = b.build().unwrap();
}

