//! Integration tests for `api/traits.rs`.
//!
//! `traits.rs` holds `pub(crate) type HttpBreakerTrait = dyn HttpBreaker`.
//! From outside the crate, its effect is that `BreakerLayer` must satisfy
//! `Send + Sync` (the supertraits of `HttpBreaker`) so it can be stored
//! behind a `dyn HttpBreaker + Send + Sync` object.

use swe_edge_egress_breaker::BreakerLayer;

/// `BreakerLayer` must be `Send`.
#[test]
fn test_breaker_layer_satisfies_send_required_by_http_breaker_trait() {
    fn assert_send<T: Send>() {}
    assert_send::<BreakerLayer>();
}

/// `BreakerLayer` must be `Sync`.
#[test]
fn test_breaker_layer_satisfies_sync_required_by_http_breaker_trait() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<BreakerLayer>();
}

/// `BreakerLayer` can be coerced to a `Box<dyn Send + Sync>` — proof that
/// the trait-object coercion the `traits.rs` alias models is possible.
#[test]
fn test_breaker_layer_coercible_to_boxed_send_sync() {
    use swe_edge_egress_breaker::{BreakerConfig, Builder};
    let cfg = BreakerConfig {
        failure_threshold: 3,
        half_open_after_seconds: 5,
        reset_after_successes: 2,
        failure_statuses: vec![500],
    };
    let layer: BreakerLayer = Builder::with_config(cfg).build().expect("build");
    let _boxed: Box<dyn Send + Sync> = Box::new(layer);
}
