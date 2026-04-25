//! Integration tests for `api/error.rs` — the public `Error` enum.
//!
//! Verifies that both variants are publicly constructable, that `Display` output
//! is actionable (names the crate + includes the reason), and that `Debug` is
//! available.

use swe_edge_http_cache::Error;

// ---------------------------------------------------------------------------
// Error::ParseFailed
// ---------------------------------------------------------------------------

/// `ParseFailed` must be publicly constructable so callers can pattern-match
/// it.  If the variant or its inner `String` field becomes private, this test
/// fails to compile.
#[test]
fn test_error_parse_failed_is_publicly_constructable() {
    let _err = Error::ParseFailed("any reason".to_string());
}

/// The `Display` output for `ParseFailed` must contain the crate name so an
/// operator reading a log line can immediately identify the source.
#[test]
fn test_error_parse_failed_display_names_crate() {
    let err = Error::ParseFailed("bad field".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("swe_edge_http_cache"),
        "ParseFailed display must name the crate; got: {msg}"
    );
}

/// The `Display` output must echo the supplied reason verbatim so the operator
/// knows exactly which TOML field or value caused the failure.
#[test]
fn test_error_parse_failed_display_echoes_reason() {
    let reason = "missing field `max_entries`";
    let err = Error::ParseFailed(reason.to_string());
    let msg = err.to_string();
    assert!(
        msg.contains(reason),
        "ParseFailed display must contain the reason string; got: {msg}"
    );
}

/// An empty reason string must still produce a non-empty `Display` message —
/// the crate name alone satisfies the "actionable" requirement.
#[test]
fn test_error_parse_failed_with_empty_reason_display_is_non_empty() {
    let err = Error::ParseFailed(String::new());
    assert!(
        !err.to_string().is_empty(),
        "ParseFailed display must not be empty even with empty reason"
    );
}

// ---------------------------------------------------------------------------
// Error::NotImplemented
// ---------------------------------------------------------------------------

/// `NotImplemented` must be publicly constructable so callers can
/// pattern-match it.
#[test]
fn test_error_not_implemented_is_publicly_constructable() {
    let _err = Error::NotImplemented("some feature");
}

/// `Display` must name the crate.
#[test]
fn test_error_not_implemented_display_names_crate() {
    let err = Error::NotImplemented("background refresh");
    let msg = err.to_string();
    assert!(
        msg.contains("swe_edge_http_cache"),
        "NotImplemented display must name the crate; got: {msg}"
    );
}

/// `Display` must not be empty — a blank error gives operators nothing to act
/// on.
#[test]
fn test_error_not_implemented_display_is_non_empty() {
    let err = Error::NotImplemented("anything");
    assert!(
        !err.to_string().is_empty(),
        "NotImplemented display must not be empty"
    );
}

// ---------------------------------------------------------------------------
// Debug — both variants
// ---------------------------------------------------------------------------

/// Both variants must implement `Debug` so they can appear in `{:?}` log
/// output without a manual trait impl.
#[test]
fn test_error_variants_implement_debug() {
    let parse_err = Error::ParseFailed("x".to_string());
    let not_impl = Error::NotImplemented("y");
    // The test fails to compile if Debug is not derived/implemented.
    let _ = format!("{parse_err:?}");
    let _ = format!("{not_impl:?}");
}

// ---------------------------------------------------------------------------
// Distinctness — the two variants must format differently
// ---------------------------------------------------------------------------

/// `ParseFailed` and `NotImplemented` must produce distinguishable `Display`
/// output so consumers can identify which error path was hit.
#[test]
fn test_error_parse_failed_and_not_implemented_display_are_distinct() {
    let a = Error::ParseFailed("foo".to_string()).to_string();
    let b = Error::NotImplemented("foo").to_string();
    assert_ne!(a, b, "the two Error variants must format differently");
}
