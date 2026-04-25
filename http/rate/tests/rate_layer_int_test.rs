//! Integration tests for `api/rate_layer.rs` — the public `RateLayer` type.
//!
//! Covers: constructability via `Builder::build()`, `Debug` output, and
//! `Send + Sync` bounds required by `reqwest_middleware::ClientBuilder::with()`.

use swe_edge_http_rate::{Builder, RateConfig, RateLayer};

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

/// The nominal construction path must succeed.
#[test]
fn test_rate_layer_builds_from_custom_config() {
    let cfg = RateConfig {
        tokens_per_second: 20,
        burst_capacity: 40,
        per_host: true,
    };
    let _layer: RateLayer = Builder::with_config(cfg)
        .build()
        .expect("build() must succeed");
}

/// Building from the SWE default must also succeed.
#[test]
fn test_rate_layer_builds_from_swe_default() {
    let _layer: RateLayer = swe_edge_http_rate::builder()
        .expect("builder() must succeed")
        .build()
        .expect("build() must succeed");
}

// ---------------------------------------------------------------------------
// Debug output
// ---------------------------------------------------------------------------

/// `RateLayer::fmt` must include the type name.
#[test]
fn test_rate_layer_debug_contains_type_name() {
    let cfg = RateConfig {
        tokens_per_second: 10,
        burst_capacity: 20,
        per_host: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("RateLayer"),
        "Debug must name the type; got: {dbg}"
    );
}

/// `tokens_per_second` must appear in `Debug` output.
#[test]
fn test_rate_layer_debug_includes_tokens_per_second() {
    let cfg = RateConfig {
        tokens_per_second: 77,
        burst_capacity: 100,
        per_host: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("77"),
        "Debug must include tokens_per_second; got: {dbg}"
    );
}

/// `burst_capacity` must appear in `Debug` output.
#[test]
fn test_rate_layer_debug_includes_burst_capacity() {
    let cfg = RateConfig {
        tokens_per_second: 5,
        burst_capacity: 333,
        per_host: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("333"),
        "Debug must include burst_capacity; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// Send + Sync — compile-time proof
// ---------------------------------------------------------------------------

/// `RateLayer` must satisfy `Send + Sync`.
#[test]
fn test_rate_layer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<RateLayer>();
}

// ---------------------------------------------------------------------------
// Two layers from different configs are independent
// ---------------------------------------------------------------------------

/// Two independently built layers must not share state.
#[test]
fn test_two_rate_layers_from_different_configs_are_independent() {
    let cfg_a = RateConfig {
        tokens_per_second: 5,
        burst_capacity: 10,
        per_host: false,
    };
    let cfg_b = RateConfig {
        tokens_per_second: 500,
        burst_capacity: 1000,
        per_host: true,
    };
    let layer_a = Builder::with_config(cfg_a).build().expect("build a");
    let layer_b = Builder::with_config(cfg_b).build().expect("build b");

    let dbg_a = format!("{layer_a:?}");
    let dbg_b = format!("{layer_b:?}");

    assert!(dbg_a.contains("5"), "layer_a must reflect rate=5; got: {dbg_a}");
    assert!(dbg_b.contains("500"), "layer_b must reflect rate=500; got: {dbg_b}");
    assert_ne!(
        dbg_a, dbg_b,
        "two layers with different configs must produce different Debug output"
    );
}
