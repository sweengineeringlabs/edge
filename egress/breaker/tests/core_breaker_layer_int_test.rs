//! Integration tests for `core/breaker_layer.rs`.
//!
//! `core::breaker_layer` contains `BreakerLayer::new`, the per-host state
//! cache construction, and the `reqwest_middleware::Middleware` impl.  All
//! internals are `pub(crate)`, so we verify observable behaviour:
//! - Various policy combinations must produce a valid layer.
//! - `Debug` output must reflect the configured policy fields.
//! - `Send + Sync` must hold after construction.

use swe_edge_egress_breaker::{BreakerConfig, BreakerLayer, Builder};

// ---------------------------------------------------------------------------
// Low threshold — breaker trips quickly
// ---------------------------------------------------------------------------

/// A threshold of 1 means the breaker opens after the very first failure.
/// `BreakerLayer::new` (called by `Builder::build`) must not reject it.
#[test]
fn test_core_breaker_layer_threshold_one_builds() {
    let cfg = BreakerConfig {
        failure_threshold: 1,
        half_open_after_seconds: 5,
        reset_after_successes: 1,
        failure_statuses: vec![500],
    };
    Builder::with_config(cfg).build().expect("failure_threshold=1 must build");
}

// ---------------------------------------------------------------------------
// Zero half-open wait — immediate probe
// ---------------------------------------------------------------------------

/// `half_open_after_seconds = 0` means the next request after opening is
/// promoted to the probe state immediately.  Must not be rejected.
#[test]
fn test_core_breaker_layer_zero_wait_builds() {
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 0,
        reset_after_successes: 2,
        failure_statuses: vec![503],
    };
    Builder::with_config(cfg).build().expect("half_open_after_seconds=0 must build");
}

// ---------------------------------------------------------------------------
// Many failure statuses — wide failure surface
// ---------------------------------------------------------------------------

/// A large `failure_statuses` slice must be accepted by `BreakerLayer::new`.
#[test]
fn test_core_breaker_layer_many_failure_statuses_builds() {
    let statuses: Vec<u16> = vec![500, 501, 502, 503, 504, 505, 506, 507, 508];
    let cfg = BreakerConfig {
        failure_threshold: 5,
        half_open_after_seconds: 30,
        reset_after_successes: 3,
        failure_statuses: statuses,
    };
    Builder::with_config(cfg).build().expect("many failure_statuses must build");
}

// ---------------------------------------------------------------------------
// Debug output reflects policy
// ---------------------------------------------------------------------------

/// `failure_threshold` must appear in `Debug` output — it is the primary
/// tuning knob operators look for in logs.
#[test]
fn test_core_breaker_layer_debug_includes_failure_threshold() {
    let cfg = BreakerConfig {
        failure_threshold: 8,
        half_open_after_seconds: 20,
        reset_after_successes: 3,
        failure_statuses: vec![500],
    };
    let layer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("8"),
        "Debug must include failure_threshold; got: {dbg}"
    );
}

/// `reset_after_successes` must appear in `Debug` output.
#[test]
fn test_core_breaker_layer_debug_includes_reset_after_successes() {
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 10,
        reset_after_successes: 6,
        failure_statuses: vec![503],
    };
    let layer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("6"),
        "Debug must include reset_after_successes; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// Send + Sync
// ---------------------------------------------------------------------------

/// `BreakerLayer` built via `BreakerLayer::new` must satisfy `Send + Sync`.
#[test]
fn test_core_breaker_layer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<BreakerLayer>();
}

// ---------------------------------------------------------------------------
// Per-host cache capacity — large host count
// ---------------------------------------------------------------------------

/// A long-running service may contact thousands of distinct hosts.  Building
/// the layer must succeed regardless of the expected host set size (the
/// in-memory host cache is bounded by moka at `MAX_TRACKED_HOSTS = 10_000`
/// internally, but the build step must not fail for any config).
#[test]
fn test_core_breaker_layer_builds_for_high_host_count_workload() {
    let cfg = BreakerConfig {
        failure_threshold: 5,
        half_open_after_seconds: 30,
        reset_after_successes: 2,
        failure_statuses: vec![500, 502, 503, 504],
    };
    Builder::with_config(cfg).build().expect("build for high-host-count workload");
}
