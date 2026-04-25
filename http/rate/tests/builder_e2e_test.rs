//! End-to-end tests for the swe_http_rate SAF builder surface.

use swe_http_rate::{Builder, RateConfig, RateLayer};

fn make_cfg() -> RateConfig {
    RateConfig { tokens_per_second: 10, burst_capacity: 20, per_host: false }
}

/// @covers: builder
#[test]
fn e2e_builder() {
    let layer: RateLayer = swe_http_rate::builder()
        .expect("builder() must succeed")
        .build()
        .expect("build() must succeed");
    assert!(format!("{layer:?}").contains("RateLayer"));
}

/// @covers: Builder::with_config
#[test]
fn e2e_with_config() {
    let b = Builder::with_config(make_cfg());
    assert_eq!(b.config().tokens_per_second, 10);
    b.build().expect("e2e with_config build must succeed");
}

/// @covers: Builder::config
#[test]
fn e2e_config() {
    let b = Builder::with_config(make_cfg());
    assert_eq!(b.config().burst_capacity, 20);
    assert!(!b.config().per_host);
}

/// @covers: Builder::build
#[test]
fn e2e_build() {
    let cfg = RateConfig { tokens_per_second: 100, burst_capacity: 500, per_host: true };
    let layer = Builder::with_config(cfg).build().expect("e2e build must succeed");
    assert!(!format!("{layer:?}").is_empty());
}
