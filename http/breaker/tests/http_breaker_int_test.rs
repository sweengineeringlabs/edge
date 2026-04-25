//! Integration tests for `api/http_breaker.rs` — the `HttpBreaker` trait.
//!
//! `HttpBreaker` is `pub(crate)`.  From outside the crate, we verify its
//! downstream effect: `BreakerLayer` must satisfy the trait's `Send + Sync`
//! supertrait bounds so it can be installed in a `reqwest_middleware::ClientBuilder`.

use swe_edge_http_breaker::{BreakerConfig, BreakerLayer, Builder};

// ---------------------------------------------------------------------------
// Send + Sync — compile-time proof that HttpBreaker's supertrait bounds hold
// ---------------------------------------------------------------------------

/// `BreakerLayer` must be `Send`.
#[test]
fn test_http_breaker_bound_send_satisfied_by_breaker_layer() {
    fn require_send<T: Send>() {}
    require_send::<BreakerLayer>();
}

/// `BreakerLayer` must be `Sync`.
#[test]
fn test_http_breaker_bound_sync_satisfied_by_breaker_layer() {
    fn require_sync<T: Sync>() {}
    require_sync::<BreakerLayer>();
}

/// Combined `Send + Sync` requirement.
#[test]
fn test_http_breaker_send_and_sync_combined_bound_satisfied() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<BreakerLayer>();
}

// ---------------------------------------------------------------------------
// Constructed layer is usable
// ---------------------------------------------------------------------------

/// A `BreakerLayer` produced by the builder must be ready to use — confirmed
/// by building and formatting it without panic.
#[test]
fn test_breaker_layer_built_from_builder_is_usable() {
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 10,
        reset_after_successes: 2,
        failure_statuses: vec![500, 503],
    };
    let layer: BreakerLayer = Builder::with_config(cfg)
        .build()
        .expect("build() must succeed");
    let dbg = format!("{layer:?}");
    assert!(!dbg.is_empty(), "BreakerLayer Debug must produce non-empty output");
}

// ---------------------------------------------------------------------------
// Arc<BreakerLayer> is also Send + Sync
// ---------------------------------------------------------------------------

/// `Arc<BreakerLayer>` must be `Send + Sync` — reqwest-middleware wraps
/// middleware in `Arc` internally.
#[test]
fn test_arc_breaker_layer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<std::sync::Arc<BreakerLayer>>();
}
