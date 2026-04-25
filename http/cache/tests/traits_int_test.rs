//! Integration tests for `api/traits.rs`.
//!
//! `traits.rs` exposes `HttpCacheTrait` as a `pub(crate)` type alias for
//! `dyn HttpCache`.  From outside the crate this is not directly accessible,
//! but we can verify its downstream effect: `CacheLayer` must satisfy all bounds
//! required to be stored behind a trait object of that form (`Send + Sync`).
//!
//! These are compile-time proofs — a runtime assertion would add noise without
//! adding information.

use swe_edge_http_cache::CacheLayer;

/// `CacheLayer` must be `Send` — required by `HttpCacheTrait = dyn HttpCache`
/// which has `HttpCache: Send + Sync` as supertraits.
#[test]
fn test_cache_layer_satisfies_send_required_by_http_cache_trait() {
    fn assert_send<T: Send>() {}
    assert_send::<CacheLayer>();
}

/// `CacheLayer` must be `Sync` — required by `HttpCacheTrait = dyn HttpCache`.
#[test]
fn test_cache_layer_satisfies_sync_required_by_http_cache_trait() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<CacheLayer>();
}

/// `CacheLayer` can be wrapped in a `Box<dyn ... + Send + Sync>` — proof that
/// the trait-object coercion the `traits.rs` alias models is possible.
#[test]
fn test_cache_layer_coercible_to_boxed_send_sync() {
    use swe_edge_http_cache::{Builder, CacheConfig};
    let cfg = CacheConfig {
        default_ttl_seconds: 5,
        max_entries: 10,
        respect_cache_control: false,
        cache_private: false,
    };
    let layer: CacheLayer = Builder::with_config(cfg).build().expect("build must succeed");
    // Coerce to a boxed `Send + Sync` object — this fails to compile if either
    // bound is absent.
    let _boxed: Box<dyn Send + Sync> = Box::new(layer);
}
