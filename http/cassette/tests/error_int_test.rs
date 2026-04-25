//! Integration tests for `swe_http_cassette::Error`.
//!
//! Covers: `Error::ParseFailed`, `Error::NotImplemented` — Display messages
//! must be actionable: name the crate, embed the payload, be non-empty.

use swe_http_cassette::Error;

// ---------------------------------------------------------------------------
// Error::ParseFailed
// ---------------------------------------------------------------------------

/// `ParseFailed` display must name the crate so an operator can trace the
/// error back to `swe_http_cassette` without reading source code.
#[test]
fn test_parse_failed_display_names_the_crate() {
    let err = Error::ParseFailed("unexpected key 'mode2'".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("swe_http_cassette"),
        "ParseFailed Display must name the crate; got: {msg}"
    );
}

/// `ParseFailed` display must echo the wrapped reason so the operator
/// knows which field or value triggered the failure.
#[test]
fn test_parse_failed_display_contains_supplied_reason() {
    let err = Error::ParseFailed("missing field `cassette_dir`".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("cassette_dir"),
        "ParseFailed Display must contain the reason; got: {msg}"
    );
}

/// `ParseFailed` must be `Debug`-printable without panicking.
#[test]
fn test_parse_failed_is_debug_printable() {
    let err = Error::ParseFailed("bad config".to_string());
    let _ = format!("{err:?}");
}

/// `ParseFailed` must be displayable as a `std::error::Error` trait object,
/// confirming the `thiserror` derive wires up the standard trait correctly.
#[test]
fn test_parse_failed_is_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(Error::ParseFailed("oops".to_string()));
    assert!(!err.to_string().is_empty());
}

// ---------------------------------------------------------------------------
// Error::NotImplemented
// ---------------------------------------------------------------------------

/// `NotImplemented` display must not be empty — a blank error gives
/// operators nothing to act on.
#[test]
fn test_not_implemented_display_is_non_empty() {
    let err = Error::NotImplemented("record strategy");
    assert!(
        !err.to_string().is_empty(),
        "NotImplemented Display must not be empty"
    );
}

/// `NotImplemented` display must name the crate so the error is traceable.
#[test]
fn test_not_implemented_display_names_the_crate() {
    let err = Error::NotImplemented("some feature");
    let msg = err.to_string();
    assert!(
        msg.contains("swe_http_cassette"),
        "NotImplemented Display must name the crate; got: {msg}"
    );
}

/// `NotImplemented` display must embed the &'static str label passed in, so
/// the operator knows which feature stub was reached.
#[test]
fn test_not_implemented_display_contains_label() {
    let err = Error::NotImplemented("replay_strategy");
    let msg = err.to_string();
    assert!(
        msg.contains("replay_strategy"),
        "NotImplemented Display must contain the label; got: {msg}"
    );
}

/// `NotImplemented` must be `Debug`-printable without panicking.
#[test]
fn test_not_implemented_is_debug_printable() {
    let err = Error::NotImplemented("builder");
    let _ = format!("{err:?}");
}

/// `NotImplemented` must be usable as a `std::error::Error` trait object.
#[test]
fn test_not_implemented_is_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(Error::NotImplemented("builder"));
    assert!(!err.to_string().is_empty());
}

// ---------------------------------------------------------------------------
// Distinct variants produce distinct messages
// ---------------------------------------------------------------------------

/// `ParseFailed` and `NotImplemented` must not produce identical messages
/// for the same payload string — callers must be able to distinguish them.
#[test]
fn test_error_variants_have_distinct_display_messages() {
    let pf = Error::ParseFailed("foo".to_string()).to_string();
    let ni = Error::NotImplemented("foo").to_string();
    assert_ne!(pf, ni, "ParseFailed and NotImplemented must produce different Display messages");
}
