//! Integration tests for `core/rate_layer.rs`.
//!
//! `core::rate_layer` contains `RateLayer::new`, `key_for`, `bucket`, and the
//! `reqwest_middleware::Middleware` impl.  All internals are `pub(crate)`.
//! We verify observable behaviour:
//! - Edge-case configs must produce a valid layer without error.
//! - `Debug` output must reflect the configured policy.
//! - `Send + Sync` must hold after construction.

use swe_http_rate::{Builder, RateConfig, RateLayer};

// ---------------------------------------------------------------------------
// Global bucket (per_host = false)
// ---------------------------------------------------------------------------

/// A layer with `per_host = false` routes all traffic through a single
/// token bucket.  Build must succeed.
#[test]
fn test_core_rate_layer_global_bucket_builds() {
    let cfg = RateConfig {
        tokens_per_second: 10,
        burst_capacity: 20,
        per_host: false,
    };
    Builder::with_config(cfg).build().expect("per_host=false (global bucket) must build");
}

/// The `Debug` output for a global-bucket layer must include `per_host: false`.
#[test]
fn test_core_rate_layer_global_bucket_visible_in_debug() {
    let cfg = RateConfig {
        tokens_per_second: 10,
        burst_capacity: 20,
        per_host: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("false"),
        "Debug must include per_host: false; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// Per-host bucket (per_host = true)
// ---------------------------------------------------------------------------

/// A layer with `per_host = true` creates separate buckets per origin.
#[test]
fn test_core_rate_layer_per_host_bucket_builds() {
    let cfg = RateConfig {
        tokens_per_second: 5,
        burst_capacity: 15,
        per_host: true,
    };
    Builder::with_config(cfg).build().expect("per_host=true must build");
}

// ---------------------------------------------------------------------------
// High token rate — no overflow in moka cache construction
// ---------------------------------------------------------------------------

/// Large token rates are valid operator choices; the underlying token bucket
/// and moka cache construction must handle them without error.
#[test]
fn test_core_rate_layer_high_rate_builds() {
    let cfg = RateConfig {
        tokens_per_second: 100_000,
        burst_capacity: 500_000,
        per_host: true,
    };
    Builder::with_config(cfg)
        .build()
        .expect("very high rate must not be rejected");
}

// ---------------------------------------------------------------------------
// Debug reflects policy fields
// ---------------------------------------------------------------------------

/// `tokens_per_second` must appear in the `Debug` output.
#[test]
fn test_core_rate_layer_debug_includes_tokens_per_second() {
    let cfg = RateConfig {
        tokens_per_second: 42,
        burst_capacity: 84,
        per_host: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("42"),
        "Debug must include tokens_per_second; got: {dbg}"
    );
}

/// `burst_capacity` must appear in the `Debug` output.
#[test]
fn test_core_rate_layer_debug_includes_burst_capacity() {
    let cfg = RateConfig {
        tokens_per_second: 10,
        burst_capacity: 777,
        per_host: true,
    };
    let layer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("777"),
        "Debug must include burst_capacity; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// Send + Sync
// ---------------------------------------------------------------------------

/// `RateLayer` built via `RateLayer::new` must satisfy `Send + Sync`.
#[test]
fn test_core_rate_layer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<RateLayer>();
}
