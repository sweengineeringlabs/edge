//! End-to-end tests for the swe_edge_http_retry SAF builder surface.

use swe_edge_http_retry::{Builder, RetryConfig, RetryLayer};

fn make_cfg() -> RetryConfig {
    RetryConfig {
        max_retries: 3,
        initial_interval_ms: 100,
        max_interval_ms: 5000,
        multiplier: 2.0,
        retryable_statuses: vec![429, 503],
        retryable_methods: vec!["GET".to_string()],
    }
}

/// @covers: builder
#[test]
fn e2e_builder() {
    let layer: RetryLayer = swe_edge_http_retry::builder()
        .expect("builder() must succeed")
        .build()
        .expect("build() must succeed");
    assert!(format!("{layer:?}").contains("RetryLayer"));
}

/// @covers: Builder::with_config
#[test]
fn e2e_with_config() {
    let b = Builder::with_config(make_cfg());
    assert_eq!(b.config().max_retries, 3);
    b.build().expect("e2e with_config build must succeed");
}

/// @covers: Builder::config
#[test]
fn e2e_config() {
    let b = Builder::with_config(make_cfg());
    assert_eq!(b.config().initial_interval_ms, 100);
    assert!(b.config().retryable_statuses.contains(&429));
}

/// @covers: Builder::build
#[test]
fn e2e_build() {
    let cfg = RetryConfig {
        max_retries: 5,
        initial_interval_ms: 50,
        max_interval_ms: 10000,
        multiplier: 1.5,
        retryable_statuses: vec![503, 504],
        retryable_methods: vec!["GET".to_string(), "HEAD".to_string()],
    };
    let layer = Builder::with_config(cfg).build().expect("e2e build must succeed");
    assert!(!format!("{layer:?}").is_empty());
}
