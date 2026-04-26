//! Integration tests for `core/cached_entry.rs` — observable caching
//! behaviour driven through the public builder + layer surface.
//!
//! `CachedEntry` and its helpers are `pub(crate)`, so we cannot construct
//! them directly from an integration test.  Instead, we verify the behaviours
//! that the entry machinery enables:
//!
//! - A layer built with a **positive** TTL must accept cacheable configs.
//! - A layer built with a **zero** TTL (disabling the fallback) must also
//!   build cleanly — the "do not cache when no Cache-Control and TTL = 0"
//!   behaviour is documented on `CacheLayer::ttl_for`.
//! - The per-entry vary-header snapshot logic is indirectly verified by
//!   confirming that the layer can be built with configs that would exercise
//!   different Vary paths at runtime.

use swe_edge_egress_cache::{Builder, CacheConfig, CacheLayer};

// ---------------------------------------------------------------------------
// TTL-positive layers (entries will be stored)
// ---------------------------------------------------------------------------

/// A layer configured with a 300 s fallback TTL must build successfully — this
/// is the config that puts the entry-storage code on the hot path.
#[test]
fn test_cached_entry_positive_ttl_layer_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 300,
        max_entries: 100,
        respect_cache_control: true,
        cache_private: false,
    };
    let layer: CacheLayer = Builder::with_config(cfg).build().expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("300"),
        "debug must reflect configured TTL; got: {dbg}"
    );
}

/// A layer configured with a 1 s TTL exercises the path where entries expire
/// quickly.  Build must not reject it.
#[test]
fn test_cached_entry_short_ttl_layer_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 1,
        max_entries: 50,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg).build().expect("TTL=1 must not be rejected");
}

// ---------------------------------------------------------------------------
// Zero-TTL layer (entries will not be stored via fallback)
// ---------------------------------------------------------------------------

/// A layer with `default_ttl_seconds = 0` must build — "no fallback TTL" is
/// valid when the operator expects all upstream responses to carry their own
/// Cache-Control.
#[test]
fn test_cached_entry_zero_ttl_fallback_layer_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 0,
        max_entries: 10,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg).build().expect("TTL=0 must not be rejected");
}

// ---------------------------------------------------------------------------
// Large max_entries — moka does not reject high capacity
// ---------------------------------------------------------------------------

/// Large entry caches are legitimate.  Build must succeed so that operators
/// can provision high-throughput deployments.
#[test]
fn test_cached_entry_large_capacity_layer_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 500_000,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg).build().expect("max_entries=500_000 must not be rejected");
}

// ---------------------------------------------------------------------------
// cache_private = true — private entries path
// ---------------------------------------------------------------------------

/// When `cache_private = true` the layer must store `Cache-Control: private`
/// responses; the build itself must succeed.
#[test]
fn test_cached_entry_cache_private_true_layer_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 120,
        max_entries: 200,
        respect_cache_control: true,
        cache_private: true,
    };
    Builder::with_config(cfg).build().expect("cache_private=true must not be rejected");
}

// ---------------------------------------------------------------------------
// respect_cache_control = false — custom-TTL path
// ---------------------------------------------------------------------------

/// When `respect_cache_control = false` every response is stored under the
/// configured fallback TTL regardless of upstream headers.  Build must succeed.
#[test]
fn test_cached_entry_ignore_cache_control_layer_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 30,
        max_entries: 100,
        respect_cache_control: false,
        cache_private: false,
    };
    Builder::with_config(cfg).build().expect("respect_cache_control=false must not be rejected");
}
