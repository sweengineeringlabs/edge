//! End-to-end tests for the swe_edge_http_breaker SAF builder surface.

use swe_edge_http_breaker::{BreakerConfig, BreakerLayer, Builder};

fn make_cfg() -> BreakerConfig {
    BreakerConfig { failure_threshold: 3, half_open_after_seconds: 60, reset_after_successes: 2, failure_statuses: vec![500, 502, 503] }
}

/// @covers: builder
#[test]
fn e2e_builder() {
    let layer: BreakerLayer = swe_edge_http_breaker::builder()
        .expect("builder() must succeed")
        .build()
        .expect("build() must succeed");
    assert!(format!("{layer:?}").contains("BreakerLayer"));
}

/// @covers: Builder::with_config
#[test]
fn e2e_with_config() {
    let b = Builder::with_config(make_cfg());
    assert_eq!(b.config().failure_threshold, 3);
    b.build().expect("e2e with_config build must succeed");
}

/// @covers: Builder::config
#[test]
fn e2e_config() {
    let b = Builder::with_config(make_cfg());
    assert_eq!(b.config().failure_statuses, vec![500u16, 502, 503]);
    assert_eq!(b.config().reset_after_successes, 2);
}

/// @covers: Builder::build
#[test]
fn e2e_build() {
    let cfg = BreakerConfig {
        failure_threshold: 5,
        half_open_after_seconds: 30,
        reset_after_successes: 3,
        failure_statuses: vec![503, 504],
    };
    let layer = Builder::with_config(cfg).build().expect("e2e build must succeed");
    assert!(!format!("{layer:?}").is_empty());
}
