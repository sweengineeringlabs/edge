//! End-to-end tests for builder/SAF facade.

#[test]
fn e2e_builder() {
    let _b = swe_http_breaker::builder().unwrap();
}

#[test]
fn e2e_with_config_parses_custom_toml_and_flows_through_to_builder() {
    let toml = r#"
        failure_threshold = 10
        half_open_after_seconds = 60
        reset_after_successes = 5
        failure_statuses = [500, 502, 503]
    "#;
    let cfg = swe_http_breaker::BreakerConfig::from_config(toml)
        .expect("from_config parses");
    assert_eq!(cfg.failure_threshold, 10);
    assert_eq!(cfg.half_open_after_seconds, 60);
    let b = swe_http_breaker::Builder::with_config(cfg);
    assert_eq!(b.config().failure_threshold, 10);
    let _layer = b.build().expect("build ok");
}

#[test]
fn e2e_config() {
    let b = swe_http_breaker::builder().unwrap();
    let _cfg = b.config();
}

#[test]
fn e2e_build() {
    let b = swe_http_breaker::builder().unwrap();
    let _layer = b.build().unwrap();
}

