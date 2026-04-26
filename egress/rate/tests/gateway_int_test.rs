//! Integration tests exercising the public gateway surface of the swe_edge_egress_rate crate.

use swe_edge_egress_rate::{Builder, Error, RateConfig, RateLayer};

fn make_cfg() -> RateConfig {
    RateConfig {
        tokens_per_second: 10,
        burst_capacity: 20,
        per_host: false,
    }
}

#[test]
fn test_builder_fn_loads_swe_default_and_succeeds() {
    swe_edge_egress_rate::builder().expect("builder() must succeed");
}

#[test]
fn test_builder_fn_default_config_has_positive_tokens_per_second() {
    let b = swe_edge_egress_rate::builder().expect("builder() must succeed");
    assert!(b.config().tokens_per_second >= 1, "swe_default tokens_per_second must be >= 1");
}

#[test]
fn test_with_config_custom_config_stores_values() {
    let b = Builder::with_config(make_cfg());
    assert_eq!(b.config().tokens_per_second, 10);
    assert_eq!(b.config().burst_capacity, 20);
    assert!(!b.config().per_host);
}

#[test]
fn test_build_default_produces_rate_layer() {
    let layer: RateLayer = swe_edge_egress_rate::builder()
        .expect("builder() must succeed")
        .build()
        .expect("build() must succeed");
    let s = format!("{layer:?}");
    assert!(s.contains("RateLayer"), "Debug must contain 'RateLayer': {s}");
}

#[test]
fn test_build_custom_config_produces_layer() {
    Builder::with_config(make_cfg()).build().expect("build with custom cfg must succeed");
}

#[test]
fn test_rate_layer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<RateLayer>();
}

#[test]
fn test_build_with_per_host_true_succeeds() {
    let cfg = RateConfig { tokens_per_second: 10, burst_capacity: 20, per_host: true };
    Builder::with_config(cfg).build().expect("per_host=true must build");
}

#[test]
fn test_build_with_per_host_false_succeeds() {
    let cfg = RateConfig { tokens_per_second: 10, burst_capacity: 20, per_host: false };
    Builder::with_config(cfg).build().expect("per_host=false must build");
}

#[test]
fn test_with_config_high_rate_flows_through_config_accessor() {
    let cfg = RateConfig { tokens_per_second: 1000, burst_capacity: 5000, per_host: false };
    let b = Builder::with_config(cfg);
    assert_eq!(b.config().tokens_per_second, 1000);
}

#[test]
fn test_error_parse_failed_display_contains_crate_name() {
    let err = Error::ParseFailed("oops".to_string());
    let s = err.to_string();
    assert!(s.contains("swe_edge_egress_rate"), "ParseFailed Display must name the crate: {s}");
}

#[test]
fn test_error_not_implemented_display_is_non_empty() {
    let s = Error::NotImplemented("test").to_string();
    assert!(!s.is_empty(), "NotImplemented Display must not be empty");
}
