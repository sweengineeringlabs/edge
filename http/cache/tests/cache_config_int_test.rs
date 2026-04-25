//! Integration tests for `api/cache_config.rs` — `CacheConfig` struct fields
//! and their observable semantics from outside the crate.

use swe_edge_http_cache::{Builder, CacheConfig};

// ---------------------------------------------------------------------------
// Struct literal construction — all four fields are public
// ---------------------------------------------------------------------------

/// Verifies that every field of `CacheConfig` is publicly constructable via a
/// struct literal.  If a field is renamed, removed, or made `pub(crate)`, this
/// test fails to compile.
#[test]
fn test_cache_config_can_be_constructed_with_all_fields() {
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 100,
        respect_cache_control: true,
        cache_private: false,
    };
    // Verify each field is readable (not just constructable).
    assert_eq!(cfg.default_ttl_seconds, 60);
    assert_eq!(cfg.max_entries, 100);
    assert!(cfg.respect_cache_control);
    assert!(!cfg.cache_private);
}

// ---------------------------------------------------------------------------
// default_ttl_seconds — boundary values
// ---------------------------------------------------------------------------

/// A zero-second TTL is legal — it means "cache only entries that supply their
/// own Cache-Control max-age".  The builder must not reject it.
#[test]
fn test_cache_config_zero_ttl_is_legal_and_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 0,
        max_entries: 10,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg).build().expect("zero TTL must not be rejected");
}

/// Large TTL values (e.g. one year in seconds) must be accepted without overflow
/// or truncation.
#[test]
fn test_cache_config_large_ttl_is_accepted() {
    const ONE_YEAR_SECS: u64 = 365 * 24 * 3600;
    let cfg = CacheConfig {
        default_ttl_seconds: ONE_YEAR_SECS,
        max_entries: 1,
        respect_cache_control: false,
        cache_private: false,
    };
    assert_eq!(cfg.default_ttl_seconds, ONE_YEAR_SECS);
    Builder::with_config(cfg).build().expect("large TTL must not be rejected");
}

// ---------------------------------------------------------------------------
// max_entries — boundary values
// ---------------------------------------------------------------------------

/// `max_entries = 1` is a legal (if tiny) cache — moka permits single-entry
/// caches.
#[test]
fn test_cache_config_single_entry_capacity_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 30,
        max_entries: 1,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg).build().expect("max_entries=1 must not be rejected");
}

// ---------------------------------------------------------------------------
// respect_cache_control flag — both values must build
// ---------------------------------------------------------------------------

/// `respect_cache_control = true` (the common case) must produce a valid layer.
#[test]
fn test_cache_config_respect_cache_control_true_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 50,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("respect_cache_control=true must build");
}

/// `respect_cache_control = false` (ignore upstream headers, always apply the
/// default TTL) is a valid override.
#[test]
fn test_cache_config_respect_cache_control_false_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 50,
        respect_cache_control: false,
        cache_private: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("respect_cache_control=false must build");
}

// ---------------------------------------------------------------------------
// cache_private flag — both values must build
// ---------------------------------------------------------------------------

/// `cache_private = false` (the privacy-safe default) must build.
#[test]
fn test_cache_config_cache_private_false_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 50,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("cache_private=false must build");
}

/// `cache_private = true` is an explicit operator opt-in; must not be rejected.
#[test]
fn test_cache_config_cache_private_true_builds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 50,
        respect_cache_control: true,
        cache_private: true,
    };
    Builder::with_config(cfg)
        .build()
        .expect("cache_private=true must build");
}

// ---------------------------------------------------------------------------
// Config round-trips through Builder.config()
// ---------------------------------------------------------------------------

/// The config stored in a `Builder` must equal what was supplied — no field
/// must be silently mutated between `with_config()` and `config()`.
#[test]
fn test_cache_config_round_trips_through_builder_unchanged() {
    let cfg = CacheConfig {
        default_ttl_seconds: 77,
        max_entries: 333,
        respect_cache_control: false,
        cache_private: true,
    };
    let b = Builder::with_config(cfg);
    let out = b.config();
    assert_eq!(out.default_ttl_seconds, 77);
    assert_eq!(out.max_entries, 333);
    assert!(!out.respect_cache_control);
    assert!(out.cache_private);
}
