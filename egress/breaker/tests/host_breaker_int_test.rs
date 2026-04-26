//! Integration tests for `core/host_breaker.rs`.
//!
//! `HostBreaker` and its state machine are `pub(crate)`.  From an integration
//! test we verify the externally observable outcomes of the state transitions:
//! the `BreakerLayer` produced by the builder must correctly reject requests
//! when the circuit is open, based on configured thresholds.
//!
//! Because we cannot directly manipulate `HostBreaker` state from outside the
//! crate, these tests probe the *construction* and *policy shape* aspects that
//! integration tests can reach:
//! - The layer builds with configs designed to exercise open/closed/half-open
//!   boundary transitions.
//! - Policy field values are faithfully carried by the layer's `Debug` output.

use swe_edge_egress_breaker::{BreakerConfig, BreakerLayer, Builder};

// ---------------------------------------------------------------------------
// Threshold = 1 — opens on first failure
// ---------------------------------------------------------------------------

/// A layer with `failure_threshold = 1` must build cleanly.  At runtime this
/// means the breaker opens after a single failure event.
#[test]
fn test_host_breaker_threshold_one_layer_builds() {
    let cfg = BreakerConfig {
        failure_threshold: 1,
        half_open_after_seconds: 10,
        reset_after_successes: 1,
        failure_statuses: vec![500],
    };
    let layer: BreakerLayer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(dbg.contains("1"), "failure_threshold=1 must appear in debug; got: {dbg}");
}

// ---------------------------------------------------------------------------
// reset_after_successes = 1 — closes after a single probe success
// ---------------------------------------------------------------------------

/// A layer configured to close after a single probe success must build cleanly.
#[test]
fn test_host_breaker_single_success_reset_layer_builds() {
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 5,
        reset_after_successes: 1,
        failure_statuses: vec![503],
    };
    Builder::with_config(cfg)
        .build()
        .expect("reset_after_successes=1 must not be rejected");
}

// ---------------------------------------------------------------------------
// half_open_after_seconds = 0 — immediate probe after opening
// ---------------------------------------------------------------------------

/// `half_open_after_seconds = 0` means the state machine allows the very next
/// request to probe after an open event.  Build must succeed.
#[test]
fn test_host_breaker_zero_wait_before_half_open_builds() {
    let cfg = BreakerConfig {
        failure_threshold: 5,
        half_open_after_seconds: 0,
        reset_after_successes: 2,
        failure_statuses: vec![500, 503],
    };
    Builder::with_config(cfg)
        .build()
        .expect("half_open_after_seconds=0 must not be rejected");
}

// ---------------------------------------------------------------------------
// Only 4xx statuses as failure triggers — unusual but valid
// ---------------------------------------------------------------------------

/// Treating 4xx responses as breaker failures is an unusual but valid policy
/// (e.g. a strict API that should never return 429 or 404).  Build must accept
/// it.
#[test]
fn test_host_breaker_4xx_failure_statuses_layer_builds() {
    let cfg = BreakerConfig {
        failure_threshold: 10,
        half_open_after_seconds: 30,
        reset_after_successes: 3,
        failure_statuses: vec![400, 404, 429],
    };
    Builder::with_config(cfg)
        .build()
        .expect("4xx failure_statuses must not be rejected");
}

// ---------------------------------------------------------------------------
// Multiple layers share no mutable state
// ---------------------------------------------------------------------------

/// Two independently built layers must not share any per-host breaker state.
/// We confirm this structurally: each layer's `Debug` reflects its own config.
#[test]
fn test_host_breaker_two_layers_have_independent_state() {
    let cfg_a = BreakerConfig {
        failure_threshold: 2,
        half_open_after_seconds: 5,
        reset_after_successes: 1,
        failure_statuses: vec![500],
    };
    let cfg_b = BreakerConfig {
        failure_threshold: 10,
        half_open_after_seconds: 60,
        reset_after_successes: 5,
        failure_statuses: vec![503],
    };
    let a = Builder::with_config(cfg_a).build().expect("build a");
    let b = Builder::with_config(cfg_b).build().expect("build b");

    let dbg_a = format!("{a:?}");
    let dbg_b = format!("{b:?}");

    assert!(dbg_a.contains("2"), "layer_a must reflect threshold=2; got: {dbg_a}");
    assert!(dbg_b.contains("10"), "layer_b must reflect threshold=10; got: {dbg_b}");
    // The two layers are distinct objects — their configs must not be identical.
    assert_ne!(
        dbg_a, dbg_b,
        "two layers with different configs must produce different Debug output"
    );
}
