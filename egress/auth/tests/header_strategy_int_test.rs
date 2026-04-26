//! Integration tests for the Header auth strategy path.
//!
//! The strategy is `pub(crate)`.  Observable effects through `Builder::build()`:
//! - Missing value_env → `Error::MissingEnvVar { name: value_env }`
//! - Invalid header name (spaces, control chars) → `Error::InvalidHeaderName`
//! - Invalid header value (CR/LF in credential) → `Error::InvalidHeaderValue`
//! - Valid name + valid credential env → build succeeds

use swe_edge_egress_auth::{AuthConfig, Builder, Error};

// ---------------------------------------------------------------------------
// Missing env var
// ---------------------------------------------------------------------------

#[test]
fn test_header_strategy_missing_value_env_returns_missing_env_var() {
    let env_name = "SWE_AUTH_HEADER_MISS_01";
    std::env::remove_var(env_name);
    let err = Builder::with_config(AuthConfig::Header {
        name: "x-api-key".into(),
        value_env: env_name.into(),
    })
    .build()
    .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, env_name),
        other => panic!("expected MissingEnvVar, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Invalid header name
// ---------------------------------------------------------------------------

#[test]
fn test_header_strategy_space_in_name_returns_invalid_header_name() {
    let env_name = "SWE_AUTH_HEADER_BADNAME_01";
    std::env::set_var(env_name, "key-value");
    let err = Builder::with_config(AuthConfig::Header {
        name: "bad name with spaces".into(), // spaces forbidden in header names
        value_env: env_name.into(),
    })
    .build()
    .unwrap_err();
    match err {
        Error::InvalidHeaderName { name, .. } => {
            assert!(
                name.contains("bad name with spaces"),
                "error must name the offending header name: {name}"
            );
        }
        other => panic!("expected InvalidHeaderName, got {other:?}"),
    }
    std::env::remove_var(env_name);
}

#[test]
fn test_header_strategy_empty_name_returns_invalid_header_name() {
    let env_name = "SWE_AUTH_HEADER_EMPTYNAME_01";
    std::env::set_var(env_name, "key-value");
    let err = Builder::with_config(AuthConfig::Header {
        name: "".into(), // empty name is invalid
        value_env: env_name.into(),
    })
    .build()
    .unwrap_err();
    // Empty string fails HeaderName::from_lowercase — either InvalidHeaderName
    // or InvalidHeaderValue is acceptable; must not succeed or return MissingEnvVar.
    assert!(
        matches!(
            err,
            Error::InvalidHeaderName { .. } | Error::InvalidHeaderValue(_)
        ),
        "empty header name must produce an error, got {err:?}"
    );
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Invalid header value (CR/LF in credential)
// ---------------------------------------------------------------------------

#[test]
fn test_header_strategy_newline_in_value_returns_invalid_header_value() {
    let env_name = "SWE_AUTH_HEADER_BADVAL_01";
    std::env::set_var(env_name, "bad\nvalue");
    let err = Builder::with_config(AuthConfig::Header {
        name: "x-api-key".into(),
        value_env: env_name.into(),
    })
    .build()
    .unwrap_err();
    assert!(
        matches!(err, Error::InvalidHeaderValue(_)),
        "newline in header value must produce InvalidHeaderValue, got {err:?}"
    );
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Happy paths
// ---------------------------------------------------------------------------

#[test]
fn test_header_strategy_lowercase_name_with_valid_value_builds_successfully() {
    let env_name = "SWE_AUTH_HEADER_OK_01";
    std::env::set_var(env_name, "api-key-value-123");
    Builder::with_config(AuthConfig::Header {
        name: "x-api-key".into(),
        value_env: env_name.into(),
    })
    .build()
    .expect("lowercase header name + valid value must build");
    std::env::remove_var(env_name);
}

#[test]
fn test_header_strategy_uppercase_name_is_accepted_via_lowercasing() {
    // The strategy lowercases the name before parsing.
    let env_name = "SWE_AUTH_HEADER_UPCASE_01";
    std::env::set_var(env_name, "some-api-key");
    Builder::with_config(AuthConfig::Header {
        name: "X-API-Key".into(), // upper-case input
        value_env: env_name.into(),
    })
    .build()
    .expect("uppercase header name must be accepted via lowercasing");
    std::env::remove_var(env_name);
}

#[test]
fn test_header_strategy_goog_api_key_name_builds_successfully() {
    let env_name = "SWE_AUTH_HEADER_GOOG_01";
    std::env::set_var(env_name, "goog-key-value");
    Builder::with_config(AuthConfig::Header {
        name: "x-goog-api-key".into(),
        value_env: env_name.into(),
    })
    .build()
    .expect("x-goog-api-key header name must build");
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Middleware wiring
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_header_strategy_middleware_wires_into_reqwest_middleware() {
    let env_name = "SWE_AUTH_HEADER_WIRE_01";
    std::env::set_var(env_name, "wire-api-key");
    let mw = Builder::with_config(AuthConfig::Header {
        name: "x-api-key".into(),
        value_env: env_name.into(),
    })
    .build()
    .expect("header build ok");
    let _client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
        .with(mw)
        .build();
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Debug — credential not leaked
// ---------------------------------------------------------------------------

#[test]
fn test_header_strategy_middleware_debug_does_not_expose_credential() {
    let env_name = "SWE_AUTH_HEADER_DBG_01";
    let secret_val = "HEADER_SECRET_VAL_UNIQUE_MARKER";
    std::env::set_var(env_name, secret_val);
    let mw = Builder::with_config(AuthConfig::Header {
        name: "x-secret-header".into(),
        value_env: env_name.into(),
    })
    .build()
    .expect("build ok");
    let s = format!("{mw:?}");
    assert!(
        !s.contains(secret_val),
        "AuthMiddleware Debug must not expose the header credential: {s}"
    );
    std::env::remove_var(env_name);
}
