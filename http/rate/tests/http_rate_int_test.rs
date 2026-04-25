//! Integration tests for `api/http_rate.rs` — the `HttpRate` trait.
//!
//! `HttpRate` is `pub(crate)`.  From outside the crate, we verify its
//! downstream effect: `RateLayer` must satisfy the trait's `Send + Sync`
//! supertrait bounds so it can be installed in a
//! `reqwest_middleware::ClientBuilder`.

use swe_http_rate::{Builder, RateConfig, RateLayer};

// ---------------------------------------------------------------------------
// Send + Sync — compile-time proof that HttpRate's supertrait bounds hold
// ---------------------------------------------------------------------------

/// `RateLayer` must be `Send`.
#[test]
fn test_http_rate_bound_send_satisfied_by_rate_layer() {
    fn require_send<T: Send>() {}
    require_send::<RateLayer>();
}

/// `RateLayer` must be `Sync`.
#[test]
fn test_http_rate_bound_sync_satisfied_by_rate_layer() {
    fn require_sync<T: Sync>() {}
    require_sync::<RateLayer>();
}

/// Combined `Send + Sync` requirement.
#[test]
fn test_http_rate_send_and_sync_combined_bound_satisfied() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<RateLayer>();
}

// ---------------------------------------------------------------------------
// Constructed layer is usable
// ---------------------------------------------------------------------------

/// A `RateLayer` produced by the builder must be ready to use.
#[test]
fn test_rate_layer_built_from_builder_is_usable() {
    let cfg = RateConfig {
        tokens_per_second: 10,
        burst_capacity: 20,
        per_host: true,
    };
    let layer: RateLayer = Builder::with_config(cfg)
        .build()
        .expect("build() must succeed");
    let dbg = format!("{layer:?}");
    assert!(!dbg.is_empty(), "RateLayer Debug must produce non-empty output");
}

// ---------------------------------------------------------------------------
// Arc<RateLayer> is also Send + Sync
// ---------------------------------------------------------------------------

/// `Arc<RateLayer>` must be `Send + Sync`.
#[test]
fn test_arc_rate_layer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<std::sync::Arc<RateLayer>>();
}
