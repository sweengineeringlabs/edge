//! Integration tests for `api/builder.rs` and `saf/builder.rs`.
//!
//! Covers the full public builder surface: the `builder()` free function
//! and the `Builder` type's `with_config`, `config`, and `build` methods.

use swe_edge_http_cache::{builder, Builder, CacheConfig, CacheLayer, Error};

// ---------------------------------------------------------------------------
// builder() free function
// ---------------------------------------------------------------------------

/// Verifies that the free `builder()` function returns `Ok` — the crate-shipped
/// TOML baseline must always be parseable.  If it panics, the crate is broken
/// before a consumer has touched a single line of config.
#[test]
fn test_builder_fn_succeeds_with_swe_default() {
    builder().expect("builder() must succeed with the crate-shipped baseline");
}

/// A `default_ttl_seconds` of zero means every response expires the instant it
/// is stored — the cache becomes a no-op.  The baseline must set a positive TTL.
#[test]
fn test_builder_fn_swe_default_ttl_is_positive() {
    let b = builder().expect("baseline parses");
    assert!(
        b.config().default_ttl_seconds >= 1,
        "swe_default default_ttl_seconds must be >= 1, got {}",
        b.config().default_ttl_seconds
    );
}

/// A `max_entries` of zero means the backing store can hold nothing.  The
/// baseline must configure a non-zero capacity.
#[test]
fn test_builder_fn_swe_default_max_entries_is_positive() {
    let b = builder().expect("baseline parses");
    assert!(
        b.config().max_entries >= 1,
        "swe_default max_entries must be >= 1, got {}",
        b.config().max_entries
    );
}

// ---------------------------------------------------------------------------
// Builder::with_config — custom policy round-trips correctly
// ---------------------------------------------------------------------------

/// `with_config` must preserve every field of the supplied `CacheConfig`
/// without silent modification.
#[test]
fn test_with_config_preserves_all_fields() {
    let cfg = CacheConfig {
        default_ttl_seconds: 42,
        max_entries: 256,
        respect_cache_control: false,
        cache_private: true,
    };
    let b = Builder::with_config(cfg);
    let policy = b.config();
    assert_eq!(policy.default_ttl_seconds, 42);
    assert_eq!(policy.max_entries, 256);
    assert!(!policy.respect_cache_control);
    assert!(policy.cache_private);
}

/// `config()` must return a reference to the stored policy — not a copy that
/// could silently diverge from the value consumed by `build()`.
#[test]
fn test_config_accessor_returns_reference_not_divergent_copy() {
    let cfg = CacheConfig {
        default_ttl_seconds: 99,
        max_entries: 7,
        respect_cache_control: true,
        cache_private: false,
    };
    let b = Builder::with_config(cfg);
    let policy: &CacheConfig = b.config();
    // Access both fields on the same borrowed reference.  If config() returned
    // an owned value, a type-system mismatch here would catch the regression.
    assert_eq!(policy.default_ttl_seconds, 99, "ttl mismatch via &CacheConfig");
    assert_eq!(policy.max_entries, 7, "max_entries mismatch via &CacheConfig");
}

// ---------------------------------------------------------------------------
// Builder::build — produces a usable CacheLayer
// ---------------------------------------------------------------------------

/// The nominal build path must succeed and return a `CacheLayer`.
#[test]
fn test_build_from_swe_default_returns_cache_layer() {
    let layer: CacheLayer = builder()
        .expect("baseline parses")
        .build()
        .expect("build() must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("CacheLayer"),
        "Debug output must identify the type; got: {dbg}"
    );
}

/// Build with a custom config must succeed and reflect the supplied TTL in the
/// `Debug` output (which the `CacheLayer::fmt` impl includes).
#[test]
fn test_build_with_custom_ttl_reflects_in_debug_output() {
    let cfg = CacheConfig {
        default_ttl_seconds: 7,
        max_entries: 10,
        respect_cache_control: true,
        cache_private: false,
    };
    let layer = Builder::with_config(cfg).build().expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("7"),
        "Debug output must include the configured TTL; got: {dbg}"
    );
}

/// `cache_private = true` is an opt-in flag that must not be rejected at build
/// time — it is a valid runtime policy choice.
#[test]
fn test_build_with_cache_private_true_succeeds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 50,
        respect_cache_control: true,
        cache_private: true,
    };
    Builder::with_config(cfg)
        .build()
        .expect("cache_private=true must not be rejected by build()");
}

/// `respect_cache_control = false` is a legitimate policy choice.  Build must
/// not reject it.
#[test]
fn test_build_with_respect_cache_control_false_succeeds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 120,
        max_entries: 500,
        respect_cache_control: false,
        cache_private: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("respect_cache_control=false must not be rejected by build()");
}

/// Very large `max_entries` values are legitimate — operators may set high
/// memory budgets. Build must not reject them.
#[test]
fn test_build_with_large_max_entries_succeeds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 30,
        max_entries: 1_000_000,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("max_entries=1_000_000 must not be rejected by build()");
}

/// `default_ttl_seconds = 0` is valid config: it means "no TTL fallback — only
/// cache responses that supply their own Cache-Control max-age."  Build must
/// not reject it.
#[test]
fn test_build_with_zero_ttl_succeeds() {
    let cfg = CacheConfig {
        default_ttl_seconds: 0,
        max_entries: 10,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("default_ttl_seconds=0 must not be rejected by build()");
}

// ---------------------------------------------------------------------------
// Error variants — compile-time coverage
// ---------------------------------------------------------------------------

/// `Error::ParseFailed` must be constructable so downstream consumers can
/// pattern-match it.  This verifies the variant is `pub` and its inner `String`
/// is accessible.
#[test]
fn test_error_parse_failed_is_constructable_and_its_message_is_accessible() {
    let reason = "missing field `max_entries`".to_string();
    let err = Error::ParseFailed(reason.clone());
    let display = err.to_string();
    assert!(
        display.contains(&reason),
        "ParseFailed display must echo the supplied reason; got: {display}"
    );
}

/// `Error::NotImplemented` must be constructable so downstream consumers can
/// match it.
#[test]
fn test_error_not_implemented_is_constructable_and_display_is_non_empty() {
    let err = Error::NotImplemented("builder");
    let display = err.to_string();
    assert!(
        !display.is_empty(),
        "NotImplemented display must not be empty"
    );
    assert!(
        display.contains("swe_edge_http_cache"),
        "NotImplemented display must identify the crate; got: {display}"
    );
}
