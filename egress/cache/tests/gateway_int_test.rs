//! Integration tests exercising the public gateway surface of the swe_edge_egress_cache crate.

use swe_edge_egress_cache::{builder, Builder, CacheConfig, CacheLayer, Error};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn make_cfg() -> CacheConfig {
    CacheConfig {
        default_ttl_seconds: 30,
        max_entries: 100,
        respect_cache_control: true,
        cache_private: false,
    }
}

// ---------------------------------------------------------------------------
// builder() — SAF entry point
// ---------------------------------------------------------------------------

#[test]
fn test_builder_fn_loads_swe_default_returns_ok() {
    // builder() must succeed: the crate-shipped baseline TOML must always parse.
    builder().expect("swe_default baseline must parse without error");
}

#[test]
fn test_builder_fn_swe_default_has_positive_max_entries() {
    // A cache with zero capacity would silently drop every response — the
    // baseline must configure a meaningful store size.
    let b = builder().expect("baseline parses");
    assert!(
        b.config().max_entries >= 1,
        "swe_default max_entries must be >= 1, got {}",
        b.config().max_entries
    );
}

#[test]
fn test_builder_fn_swe_default_has_positive_ttl() {
    // A zero-second TTL means every cached response expires immediately,
    // making the cache useless.
    let b = builder().expect("baseline parses");
    assert!(
        b.config().default_ttl_seconds >= 1,
        "swe_default default_ttl_seconds must be >= 1, got {}",
        b.config().default_ttl_seconds
    );
}

// ---------------------------------------------------------------------------
// builder().build() — finalisation
// ---------------------------------------------------------------------------

#[test]
fn test_build_from_swe_default_returns_cache_layer() {
    // The full happy path: default config → Builder → CacheLayer.
    let layer = builder()
        .expect("baseline parses")
        .build()
        .expect("build must succeed");
    let dbg = format!("{layer:?}");
    assert!(
        dbg.contains("CacheLayer"),
        "Debug output must name the type; got: {dbg}"
    );
}

// ---------------------------------------------------------------------------
// CacheLayer: Send + Sync — compile-time proof
// ---------------------------------------------------------------------------

#[test]
fn test_cache_layer_satisfies_send_and_sync_bounds() {
    // This test fails to compile if CacheLayer stops being Send + Sync.
    // No runtime assertion needed — the compile itself is the assertion.
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<CacheLayer>();
}

// ---------------------------------------------------------------------------
// Builder::with_config — custom CacheConfig flows through correctly
// ---------------------------------------------------------------------------

#[test]
fn test_builder_with_config_stores_custom_ttl_and_max_entries() {
    let cfg = make_cfg();
    let b = Builder::with_config(cfg);
    assert_eq!(
        b.config().default_ttl_seconds,
        30,
        "custom ttl must be stored unmodified"
    );
    assert_eq!(
        b.config().max_entries,
        100,
        "custom max_entries must be stored unmodified"
    );
}

#[test]
fn test_builder_with_config_stores_respect_cache_control_flag() {
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 50,
        respect_cache_control: false, // non-default value
        cache_private: false,
    };
    let b = Builder::with_config(cfg);
    assert!(
        !b.config().respect_cache_control,
        "respect_cache_control=false must be preserved through Builder"
    );
}

#[test]
fn test_builder_with_cache_private_true_builds_successfully() {
    // Caching private responses is opt-in — must not be rejected at build.
    let cfg = CacheConfig {
        default_ttl_seconds: 120,
        max_entries: 200,
        respect_cache_control: true,
        cache_private: true,
    };
    Builder::with_config(cfg)
        .build()
        .expect("cache_private=true must produce a valid CacheLayer");
}

#[test]
fn test_builder_with_respect_cache_control_false_builds_successfully() {
    // Ignoring upstream Cache-Control is a legitimate policy choice.
    let cfg = CacheConfig {
        default_ttl_seconds: 300,
        max_entries: 1000,
        respect_cache_control: false,
        cache_private: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("respect_cache_control=false must produce a valid CacheLayer");
}

#[test]
fn test_builder_with_very_large_max_entries_builds_successfully() {
    // Operator-supplied large cache sizes must not be rejected by the builder.
    let cfg = CacheConfig {
        default_ttl_seconds: 60,
        max_entries: 1_000_000,
        respect_cache_control: true,
        cache_private: false,
    };
    Builder::with_config(cfg)
        .build()
        .expect("max_entries=1_000_000 must produce a valid CacheLayer");
}

#[test]
fn test_builder_config_accessor_returns_reference_to_stored_policy() {
    // config() must return a reference to the policy, not a copy that could
    // diverge from the one used during build.
    let cfg = make_cfg();
    let b = Builder::with_config(cfg);
    let policy: &CacheConfig = b.config();
    assert_eq!(policy.max_entries, 100);
    assert_eq!(policy.default_ttl_seconds, 30);
}

// ---------------------------------------------------------------------------
// Error variants — Display must be actionable
// ---------------------------------------------------------------------------

#[test]
fn test_error_parse_failed_display_contains_crate_name() {
    // Consumers catching Error::ParseFailed must be able to identify which
    // middleware produced the error without reading source code.
    let err = Error::ParseFailed("x".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("swe_edge_egress_cache"),
        "ParseFailed display must name the crate; got: {msg}"
    );
}

#[test]
fn test_error_parse_failed_display_contains_supplied_reason() {
    // The wrapped reason must appear verbatim so the operator knows exactly
    // which field or value triggered the failure.
    let err = Error::ParseFailed("missing field `max_entries`".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("max_entries"),
        "ParseFailed display must echo the reason; got: {msg}"
    );
}

#[test]
fn test_error_not_implemented_display_is_non_empty_and_names_crate() {
    // A blank or opaque error message leaves operators with no actionable
    // information when the scaffold-phase feature is reached at runtime.
    let err = Error::NotImplemented("middleware hook");
    let msg = err.to_string();
    assert!(!msg.is_empty(), "NotImplemented display must not be empty");
    assert!(
        msg.contains("swe_edge_egress_cache"),
        "NotImplemented display must name the crate; got: {msg}"
    );
}
