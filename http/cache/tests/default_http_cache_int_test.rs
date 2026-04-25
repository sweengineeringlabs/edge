//! Integration tests for `core/default_http_cache.rs`.
//!
//! `DefaultHttpCache` is `pub(crate)`, so we cannot name or construct it
//! directly from an integration test.  Its observable effect is through the
//! SAF `builder()` function, which loads the crate-shipped SWE baseline TOML
//! and returns a `Builder` that carries that default policy.
//!
//! These tests verify that the default values produced by the SWE baseline are
//! sane — if `DefaultHttpCache::new` or the underlying config ever regresses,
//! these assertions catch it.

use swe_edge_http_cache::{Builder, CacheConfig};

// ---------------------------------------------------------------------------
// SWE baseline — verify default config has production-safe values
// ---------------------------------------------------------------------------

/// The `builder()` function must load the crate-shipped baseline without
/// returning an error.  A failure here means the embedded TOML is malformed
/// or the config schema changed without updating the file.
#[test]
fn test_default_http_cache_swe_default_builder_succeeds() {
    swe_edge_http_cache::builder().expect("swe_default baseline must parse without error");
}

/// The default `default_ttl_seconds` must be positive — a zero-second TTL
/// would make the default configuration a silently broken no-op.
#[test]
fn test_default_http_cache_swe_default_ttl_is_positive() {
    let b = swe_edge_http_cache::builder().expect("baseline parses");
    assert!(
        b.config().default_ttl_seconds >= 1,
        "swe_default default_ttl_seconds must be >= 1, got {}",
        b.config().default_ttl_seconds
    );
}

/// The default `max_entries` must be positive — a zero-capacity store silently
/// discards every response.
#[test]
fn test_default_http_cache_swe_default_max_entries_is_positive() {
    let b = swe_edge_http_cache::builder().expect("baseline parses");
    assert!(
        b.config().max_entries >= 1,
        "swe_default max_entries must be >= 1, got {}",
        b.config().max_entries
    );
}

/// Building from the SWE default must produce a valid `CacheLayer`.
#[test]
fn test_default_http_cache_swe_default_builds_cache_layer() {
    swe_edge_http_cache::builder()
        .expect("baseline parses")
        .build()
        .expect("build from swe_default must succeed");
}

// ---------------------------------------------------------------------------
// Custom config vs SWE default — policy independence
// ---------------------------------------------------------------------------

/// A consumer-supplied config with different values from the SWE default must
/// not be silently overwritten by the default-loading path.
#[test]
fn test_default_http_cache_custom_config_is_not_overridden_by_swe_default() {
    // Use values that are deliberately different from any likely SWE default.
    let custom = CacheConfig {
        default_ttl_seconds: 3,
        max_entries: 7,
        respect_cache_control: false,
        cache_private: true,
    };
    let b = Builder::with_config(custom);
    assert_eq!(
        b.config().default_ttl_seconds,
        3,
        "custom TTL must not be overridden by the SWE default"
    );
    assert_eq!(
        b.config().max_entries,
        7,
        "custom max_entries must not be overridden by the SWE default"
    );
    assert!(
        !b.config().respect_cache_control,
        "custom respect_cache_control must not be overridden"
    );
    assert!(
        b.config().cache_private,
        "custom cache_private must not be overridden"
    );
}
