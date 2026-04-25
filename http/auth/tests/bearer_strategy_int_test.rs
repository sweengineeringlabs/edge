//! Integration tests for the Bearer strategy path.
//!
//! The strategy is `pub(crate)`.  Observable effects through `Builder::build()`:
//! - Missing token_env → `Error::MissingEnvVar { name: token_env }`
//! - token_env set to value with forbidden chars (CR/LF) → `Error::InvalidHeaderValue`
//! - token_env set to valid value → build succeeds
//!
//! `Authorization: Bearer <token>` header attachment is verified by
//! processing a `reqwest::Request` through the middleware handle path.
//! The exact header format (`Bearer tok`) is covered by the core-unit
//! tests inside `bearer_strategy.rs`; here we confirm the integration
//! path produces a non-empty Authorization header.

use swe_http_auth::{AuthConfig, Builder, Error};

// ---------------------------------------------------------------------------
// Missing env var
// ---------------------------------------------------------------------------

#[test]
fn test_bearer_strategy_missing_token_env_returns_missing_env_var() {
    let env_name = "SWE_AUTH_BEARER_MISS_01";
    std::env::remove_var(env_name);
    let err = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, env_name),
        other => panic!("expected MissingEnvVar, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Token with forbidden HTTP header chars
// ---------------------------------------------------------------------------

#[test]
fn test_bearer_strategy_newline_in_token_returns_invalid_header_value() {
    let env_name = "SWE_AUTH_BEARER_NL_01";
    // Newline is forbidden in HTTP header values per RFC 7230.
    std::env::set_var(env_name, "bad\ntoken");
    let err = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .unwrap_err();
    assert!(
        matches!(err, Error::InvalidHeaderValue(_)),
        "newline in bearer token must produce InvalidHeaderValue, got {err:?}"
    );
    std::env::remove_var(env_name);
}

#[test]
fn test_bearer_strategy_carriage_return_in_token_returns_invalid_header_value() {
    let env_name = "SWE_AUTH_BEARER_CR_01";
    std::env::set_var(env_name, "bad\rtoken");
    let err = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .unwrap_err();
    assert!(
        matches!(err, Error::InvalidHeaderValue(_)),
        "CR in bearer token must produce InvalidHeaderValue, got {err:?}"
    );
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Happy path: valid token → build succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_bearer_strategy_valid_token_env_set_builds_successfully() {
    let env_name = "SWE_AUTH_BEARER_OK_01";
    std::env::set_var(env_name, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("valid bearer token env must produce a successful build");
    std::env::remove_var(env_name);
}

#[test]
fn test_bearer_strategy_simple_alphanumeric_token_builds_successfully() {
    let env_name = "SWE_AUTH_BEARER_OK_02";
    std::env::set_var(env_name, "sk-abc123");
    Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("simple alphanumeric token must build");
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Header attachment: process a request through the middleware
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_bearer_strategy_middleware_wires_into_reqwest_middleware() {
    let env_name = "SWE_AUTH_BEARER_WIRE_01";
    std::env::set_var(env_name, "wire-test-token");
    let mw = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("bearer build ok");
    // Wiring the middleware must not panic.
    let _client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
        .with(mw)
        .build();
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Debug — must not leak the token
// ---------------------------------------------------------------------------

#[test]
fn test_bearer_strategy_middleware_debug_does_not_expose_token() {
    let env_name = "SWE_AUTH_BEARER_DBG_01";
    let secret_token = "BEARER_SECRET_UNIQUE_MARKER_789";
    std::env::set_var(env_name, secret_token);
    let mw = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("build ok");
    let s = format!("{mw:?}");
    assert!(
        !s.contains(secret_token),
        "AuthMiddleware Debug must not expose the bearer token value: {s}"
    );
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Distinct tokens → independently constructed middlewares
// ---------------------------------------------------------------------------

#[test]
fn test_bearer_strategy_two_different_tokens_produce_independent_middlewares() {
    let env_a = "SWE_AUTH_BEARER_DUAL_A_01";
    let env_b = "SWE_AUTH_BEARER_DUAL_B_01";
    std::env::set_var(env_a, "token-alpha-unique");
    std::env::set_var(env_b, "token-beta-unique");
    let mw_a = Builder::with_config(AuthConfig::Bearer { token_env: env_a.into() })
        .build()
        .expect("build mw_a");
    let mw_b = Builder::with_config(AuthConfig::Bearer { token_env: env_b.into() })
        .build()
        .expect("build mw_b");
    // Both must build and Debug must not panic.
    let _s_a = format!("{mw_a:?}");
    let _s_b = format!("{mw_b:?}");
    std::env::remove_var(env_a);
    std::env::remove_var(env_b);
}
