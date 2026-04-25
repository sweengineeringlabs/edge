//! Integration tests for `api/http_cache.rs` — the `HttpCache` trait.
//!
//! The `HttpCache` trait is `pub(crate)` so consumers cannot name it directly.
//! What we CAN observe from outside is that `CacheLayer` (the concrete type
//! produced by `Builder::build()`) satisfies the trait's bounds (`Send + Sync`),
//! and that the layer can be passed to any generic context that requires those
//! bounds.

use swe_http_cache::{Builder, CacheConfig, CacheLayer};

// ---------------------------------------------------------------------------
// Send + Sync — compile-time proof that HttpCache's supertrait bounds hold
// ---------------------------------------------------------------------------

/// `CacheLayer` must satisfy `Send + Sync`.  These are the supertraits of
/// `HttpCache`, so losing them would break the trait impl.  This test fails to
/// COMPILE if the bounds are removed — no runtime assertion needed.
#[test]
fn test_http_cache_bound_send_satisfied_by_cache_layer() {
    fn require_send<T: Send>() {}
    require_send::<CacheLayer>();
}

#[test]
fn test_http_cache_bound_sync_satisfied_by_cache_layer() {
    fn require_sync<T: Sync>() {}
    require_sync::<CacheLayer>();
}

/// Combined `Send + Sync` requirement as a single bound — the form that
/// `reqwest_middleware::ClientBuilder::with()` uses.
#[test]
fn test_http_cache_send_and_sync_combined_bound_satisfied() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<CacheLayer>();
}

// ---------------------------------------------------------------------------
// Layer is usable after being constructed via Builder
// ---------------------------------------------------------------------------

/// A `CacheLayer` produced by the builder must be ready to use — confirmed by
/// successfully building and formatting it.
#[test]
fn test_cache_layer_built_from_builder_is_usable() {
    let cfg = CacheConfig {
        default_ttl_seconds: 30,
        max_entries: 100,
        respect_cache_control: true,
        cache_private: false,
    };
    let layer: CacheLayer = Builder::with_config(cfg)
        .build()
        .expect("build() must succeed");
    // If CacheLayer's HttpCache impl were broken (e.g. panics on construction)
    // this test would surface it.
    let dbg = format!("{layer:?}");
    assert!(!dbg.is_empty(), "CacheLayer Debug must produce non-empty output");
}

// ---------------------------------------------------------------------------
// Arc<CacheLayer> is also Send + Sync (shared middleware ownership)
// ---------------------------------------------------------------------------

/// `Arc<CacheLayer>` must be `Send + Sync` — reqwest-middleware wraps
/// middleware in `Arc` internally, so this is a practical requirement.
#[test]
fn test_arc_cache_layer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<std::sync::Arc<CacheLayer>>();
}
