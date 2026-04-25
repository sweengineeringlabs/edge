//! Integration tests for `api/traits.rs`.
//!
//! `traits.rs` holds `pub(crate) type HttpRateTrait = dyn HttpRate`.
//! From outside the crate, its effect is that `RateLayer` must satisfy
//! `Send + Sync` (the supertraits of `HttpRate`) so it can be stored behind a
//! `dyn HttpRate + Send + Sync` object.

use swe_edge_http_rate::RateLayer;

/// `RateLayer` must be `Send`.
#[test]
fn test_rate_layer_satisfies_send_required_by_http_rate_trait() {
    fn assert_send<T: Send>() {}
    assert_send::<RateLayer>();
}

/// `RateLayer` must be `Sync`.
#[test]
fn test_rate_layer_satisfies_sync_required_by_http_rate_trait() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<RateLayer>();
}

/// `RateLayer` can be coerced to a `Box<dyn Send + Sync>`.
#[test]
fn test_rate_layer_coercible_to_boxed_send_sync() {
    use swe_edge_http_rate::{Builder, RateConfig};
    let cfg = RateConfig {
        tokens_per_second: 5,
        burst_capacity: 10,
        per_host: false,
    };
    let layer: RateLayer = Builder::with_config(cfg).build().expect("build");
    let _boxed: Box<dyn Send + Sync> = Box::new(layer);
}
