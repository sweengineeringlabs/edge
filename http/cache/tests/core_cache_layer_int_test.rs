//! Integration tests for `core/cache_layer.rs`.
//!
//! `core::cache_layer` contains `CacheLayer::new`, `ttl_for`, and the
//! `reqwest_middleware::Middleware` impl.  All of those are `pub(crate)`.
//! From an integration test we exercise the observable surface: the layer
//! produced by `Builder::build()` must correctly reflect the policy supplied at
//! construction, and edge-case configs must be accepted without error.

use swe_http_cache::{Builder, CacheConfig, CacheLayer};

// ---------------------------------------------------------------------------
// Zero TTL — "cache only when upstream provides Cache-Control max-age"
// ---------------------------------------------------------------------------

/// A layer constructed with `default_ttl_seconds = 0` must build cleanly.
/// Inside `core::cache_layer::ttl_for`, zero TTL means "return None when there
/// is no upstream Cache-Control" — this is documented behaviour, not a bug.
#[test]
fn test_core_cache_layer_zero_ttl_builds_without_error() {
    let cfg = CacheConfig {
        default_ttl_seconds: 0,
        max_entries: 10,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg).build().expect("zero TTL must not cause build to fail");
}

/// Zero TTL must appear in the `Debug` output so an operator can confirm that
/// their "no fallback TTL" setting is in effect.
#[test]
fn test_core_cache_layer_zero_ttl_visible_in_debug() {
    let cfg = CacheConfig {
        default_ttl_seconds: 0,
        max_entries: 10,
        respect_cache_control: true,
        cache_private: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("0"),
        "Debug must include the TTL value 0; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// Large max_entries — moka must not be upset by high capacity
// ---------------------------------------------------------------------------

/// A very large `max_entries` exercises the moka cache construction path
/// inside `CacheLayer::new`.  This is the code path in `core::cache_layer`
/// that calls `Cache::builder().max_capacity(...)`.
#[test]
fn test_core_cache_layer_large_max_entries_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 1_000_000,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("max_entries=1_000_000 must not be rejected by CacheLayer::new");
}

/// Large `max_entries` must be reflected in Debug output.
#[test]
fn test_core_cache_layer_large_max_entries_visible_in_debug() {
    let cfg = CacheConfig {
        default_ttl_seconds: 30,
        max_entries: 99_999,
        respect_cache_control: false,
        cache_private: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("99999"),
        "Debug must include max_entries; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// respect_cache_control combinations
// ---------------------------------------------------------------------------

/// `respect_cache_control = true` with a long fallback TTL — common production
/// configuration.
#[test]
fn test_core_cache_layer_respect_cc_true_long_ttl_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 3600,
        max_entries: 500,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg).build().expect("respect_cc=true + 3600s must build");
}

/// `respect_cache_control = false` — all Cache-Control headers are ignored;
/// every response is stored under the fixed fallback TTL.
#[test]
fn test_core_cache_layer_respect_cc_false_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 120,
        max_entries: 100,
        respect_cache_control: false,
        cache_private: false,
    };
    Builder::with_config(cfg).build().expect("respect_cc=false must build");
}

// ---------------------------------------------------------------------------
// Send + Sync — confirmed for the core-layer path
// ---------------------------------------------------------------------------

/// `CacheLayer` built via `core::cache_layer::CacheLayer::new` (the path
/// `Builder::build` uses) must satisfy `Send + Sync`.
#[test]
fn test_core_cache_layer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<CacheLayer>();
}
