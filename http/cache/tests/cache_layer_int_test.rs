//! Integration tests for `api/cache_layer.rs` — the public `CacheLayer` type.
//!
//! Covers: constructability via `Builder::build()`, `Debug` output, and the
//! `Send + Sync` bounds that let the layer be passed to
//! `reqwest_middleware::ClientBuilder::with()`.

use swe_http_cache::{Builder, CacheConfig, CacheLayer};

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

/// The primary construction path must produce a `CacheLayer` without error.
#[test]
fn test_cache_layer_builds_from_custom_config() {
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 200,
        respect_cache_control: true,
        cache_private: false,
    };
    let _layer: CacheLayer = Builder::with_config(cfg)
        .build()
        .expect("build() must succeed");
}

/// Building from the crate-shipped SWE default must also succeed.
#[test]
fn test_cache_layer_builds_from_swe_default() {
    let _layer: CacheLayer = swe_http_cache::builder()
        .expect("builder() must succeed")
        .build()
        .expect("build() must succeed");
}

// ---------------------------------------------------------------------------
// Debug output
// ---------------------------------------------------------------------------

/// `CacheLayer::fmt` must include the type name so log lines are parseable.
#[test]
fn test_cache_layer_debug_contains_type_name() {
    let cfg = CacheConfig {
        default_ttl_seconds: 30,
        max_entries: 50,
        respect_cache_control: true,
        cache_private: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("CacheLayer"),
        "Debug output must name the type; got: {dbg}"
    );
}

/// The `Debug` impl must include `default_ttl_seconds` so operators can
/// correlate log output with their TOML config.
#[test]
fn test_cache_layer_debug_includes_ttl_seconds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 123,
        max_entries: 10,
        respect_cache_control: false,
        cache_private: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("123"),
        "Debug output must include default_ttl_seconds; got: {dbg}"
    );
}

/// The `Debug` impl must include `max_entries` so operators can confirm their
/// capacity setting is in effect.
#[test]
fn test_cache_layer_debug_includes_max_entries() {
    let cfg = CacheConfig {
        default_ttl_seconds: 10,
        max_entries: 999,
        respect_cache_control: true,
        cache_private: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("999"),
        "Debug output must include max_entries; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// Send + Sync — compile-time proof
// ---------------------------------------------------------------------------

/// `CacheLayer` must satisfy `Send + Sync` so it can be installed into a
/// `reqwest_middleware::ClientBuilder`.  This test fails to COMPILE if the
/// bounds are lost — no runtime assertion is needed.
#[test]
fn test_cache_layer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<CacheLayer>();
}

// ---------------------------------------------------------------------------
// Varied configs produce independently valid layers
// ---------------------------------------------------------------------------

/// Building two layers from different configs must produce independent objects —
/// there must be no shared mutable state that would let one layer's policy bleed
/// into the other.
#[test]
fn test_two_layers_from_different_configs_are_independent() {
    let cfg_a = CacheConfig {
        default_ttl_seconds: 10,
        max_entries: 100,
        respect_cache_control: true,
        cache_private: false,
    };
    let cfg_b = CacheConfig {
        default_ttl_seconds: 600,
        max_entries: 5,
        respect_cache_control: false,
        cache_private: true,
    };
    let layer_a = Builder::with_config(cfg_a).build().expect("build a");
    let layer_b = Builder::with_config(cfg_b).build().expect("build b");

    let dbg_a = format!("{layer_a:?}");
    let dbg_b = format!("{layer_b:?}");

    // Each layer's Debug output must reflect its own config, not the other's.
    assert!(
        dbg_a.contains("10"),
        "layer_a debug must show ttl=10; got: {dbg_a}"
    );
    assert!(
        dbg_b.contains("600"),
        "layer_b debug must show ttl=600; got: {dbg_b}"
    );
}
