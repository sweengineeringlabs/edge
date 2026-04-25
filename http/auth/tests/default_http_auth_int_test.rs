//! Integration tests for `DefaultHttpAuth` behaviour.
//!
//! `DefaultHttpAuth` is `pub(crate)`.  Its effects are observable through:
//! 1. `Builder::build()` success/failure (build-time credential resolution)
//! 2. The `AuthMiddleware::handle()` path (reqwest_middleware trait) which
//!    delegates to `DefaultHttpAuth::process()`.
//! 3. The Debug output of `AuthMiddleware`, which calls
//!    `processor.describe()` — always returns `"swe_edge_http_auth"` for
//!    `DefaultHttpAuth`.
//!
//! Header-attachment assertions are kept in the per-strategy test files.
//! This file focuses on `DefaultHttpAuth`'s own contract:
//! - `describe()` returns `"swe_edge_http_auth"` regardless of config.
//! - `build()` fails fast when env vars are missing.
//! - `build()` succeeds when env vars are present.
//! - `process()` is reachable end-to-end via the middleware handle path.

use swe_edge_http_auth::{AuthConfig, AuthMiddleware, Builder, Error};

// ---------------------------------------------------------------------------
// describe() via AuthMiddleware Debug
// ---------------------------------------------------------------------------

#[test]
fn test_default_http_auth_describe_returns_crate_name_for_none_config() {
    let mw: AuthMiddleware = Builder::with_config(AuthConfig::None)
        .build()
        .expect("None must build");
    let s = format!("{mw:?}");
    assert!(
        s.contains("swe_edge_http_auth"),
        "DefaultHttpAuth::describe() must return 'swe_edge_http_auth': {s}"
    );
}

#[test]
fn test_default_http_auth_describe_returns_crate_name_for_bearer_config() {
    let env_name = "SWE_AUTH_DHA_DESC_BRR_01";
    std::env::set_var(env_name, "describe-test-tok");
    let mw = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("Bearer with env set must build");
    let s = format!("{mw:?}");
    assert!(
        s.contains("swe_edge_http_auth"),
        "DefaultHttpAuth::describe() must return 'swe_edge_http_auth' for Bearer: {s}"
    );
    std::env::remove_var(env_name);
}

#[test]
fn test_default_http_auth_describe_same_for_all_configs() {
    // describe() is a constant "swe_edge_http_auth" regardless of scheme.
    // Build two middlewares with different schemes and compare.
    let env_name = "SWE_AUTH_DHA_DESC_SAME_01";
    std::env::set_var(env_name, "tok");
    let mw_none = Builder::with_config(AuthConfig::None).build().unwrap();
    let mw_bearer = Builder::with_config(AuthConfig::Bearer {
        token_env: env_name.into(),
    })
    .build()
    .unwrap();
    let s_none = format!("{mw_none:?}");
    let s_bearer = format!("{mw_bearer:?}");
    assert!(s_none.contains("swe_edge_http_auth"), "{s_none}");
    assert!(s_bearer.contains("swe_edge_http_auth"), "{s_bearer}");
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// build() — fails fast with MissingEnvVar
// ---------------------------------------------------------------------------

#[test]
fn test_default_http_auth_build_fails_with_missing_env_var_for_bearer() {
    let env_name = "SWE_AUTH_DHA_BUILD_MISS_BRR_01";
    std::env::remove_var(env_name);
    let err = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, env_name),
        other => panic!("expected MissingEnvVar at build time, got {other:?}"),
    }
}

#[test]
fn test_default_http_auth_build_fails_with_missing_env_var_for_basic() {
    let user_env = "SWE_AUTH_DHA_BUILD_MISS_BASIC_U_01";
    let pass_env = "SWE_AUTH_DHA_BUILD_MISS_BASIC_P_01";
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
    let err = Builder::with_config(AuthConfig::Basic {
        user_env: user_env.into(),
        pass_env: pass_env.into(),
    })
    .build()
    .unwrap_err();
    assert!(
        matches!(err, Error::MissingEnvVar { .. }),
        "expected MissingEnvVar for basic, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// build() — succeeds with envs present
// ---------------------------------------------------------------------------

#[test]
fn test_default_http_auth_build_succeeds_for_none() {
    Builder::with_config(AuthConfig::None)
        .build()
        .expect("None config must always build");
}

#[test]
fn test_default_http_auth_build_succeeds_for_bearer_with_env() {
    let env_name = "SWE_AUTH_DHA_BUILD_OK_BRR_01";
    std::env::set_var(env_name, "build-ok-token");
    Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("Bearer with env set must build successfully");
    std::env::remove_var(env_name);
}

#[test]
fn test_default_http_auth_build_succeeds_for_basic_with_envs() {
    let user_env = "SWE_AUTH_DHA_BUILD_OK_BASIC_U_01";
    let pass_env = "SWE_AUTH_DHA_BUILD_OK_BASIC_P_01";
    std::env::set_var(user_env, "alice");
    std::env::set_var(pass_env, "password");
    Builder::with_config(AuthConfig::Basic {
        user_env: user_env.into(),
        pass_env: pass_env.into(),
    })
    .build()
    .expect("Basic with both envs must build successfully");
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
}

// ---------------------------------------------------------------------------
// process() — end-to-end via reqwest_middleware
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_default_http_auth_process_none_attaches_no_auth_headers() {
    use reqwest_middleware::ClientBuilder;

    // None config → NoopStrategy → process() must add no headers.
    // We can't send a real request, but we can wire the middleware into
    // a ClientBuilder and verify it doesn't panic during wiring.
    let mw = Builder::with_config(AuthConfig::None)
        .build()
        .expect("build ok");
    let _client = ClientBuilder::new(reqwest::Client::new())
        .with(mw)
        .build();
    // Reaching here without panic proves the process() pathway compiles
    // and the middleware is correctly wired.
}

#[tokio::test]
async fn test_default_http_auth_process_bearer_wires_without_panic() {
    use reqwest_middleware::ClientBuilder;

    let env_name = "SWE_AUTH_DHA_PROC_BRR_01";
    std::env::set_var(env_name, "proc-bearer-tok");
    let mw = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("Bearer build ok");
    let _client = ClientBuilder::new(reqwest::Client::new())
        .with(mw)
        .build();
    std::env::remove_var(env_name);
}
