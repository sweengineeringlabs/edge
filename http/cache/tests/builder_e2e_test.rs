//! End-to-end tests for the swe_edge_http_cache SAF builder surface.

use swe_edge_http_cache::{Builder, CacheConfig, CacheLayer};

fn make_cfg() -> CacheConfig {
    CacheConfig { default_ttl_seconds: 300, max_entries: 100, respect_cache_control: true, cache_private: false }
}

/// @covers: builder
#[test]
fn e2e_builder() {
    let layer: CacheLayer = swe_edge_http_cache::builder()
        .expect("builder() must succeed")
        .build()
        .expect("build() must succeed");
    let s = format!("{layer:?}");
    assert!(s.contains("CacheLayer"), "e2e: Debug must contain 'CacheLayer': {s}");
}

/// @covers: Builder::with_config
#[test]
fn e2e_with_config() {
    let b = Builder::with_config(make_cfg());
    assert_eq!(b.config().default_ttl_seconds, 300);
    b.build().expect("e2e with_config build must succeed");
}

/// @covers: Builder::config
#[test]
fn e2e_config() {
    let b = Builder::with_config(make_cfg());
    assert_eq!(b.config().max_entries, 100);
    assert!(b.config().respect_cache_control);
}

/// @covers: Builder::build
#[test]
fn e2e_build() {
    let cfg = CacheConfig { default_ttl_seconds: 60, max_entries: 50, respect_cache_control: false, cache_private: true };
    let layer = Builder::with_config(cfg).build().expect("e2e build must succeed");
    assert!(!format!("{layer:?}").is_empty());
}
