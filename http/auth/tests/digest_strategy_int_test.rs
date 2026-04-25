//! Integration tests for the Digest auth strategy path.
//!
//! The strategy is `pub(crate)`.  Observable effects through `Builder::build()`:
//! - Missing user_env → `Error::MissingEnvVar { name: user_env }`
//! - Missing password_env (when user_env present) → MissingEnvVar
//! - Both envs set → build succeeds (strategy is constructed)
//! - realm is optional — None and Some("…") both build
//!
//! The Digest protocol's nonce-fetching logic requires a live server
//! and cannot be tested without one.  Integration tests here are
//! confined to what is verifiable without network access.

use swe_edge_http_auth::{AuthConfig, Builder, Error};

// ---------------------------------------------------------------------------
// Missing env vars
// ---------------------------------------------------------------------------

#[test]
fn test_digest_strategy_missing_user_env_returns_missing_env_var() {
    let user_env = "SWE_AUTH_DIGEST_MISS_U_01";
    let pass_env = "SWE_AUTH_DIGEST_MISS_P_01";
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
    let err = Builder::with_config(AuthConfig::Digest {
        user_env: user_env.into(),
        password_env: pass_env.into(),
        realm: None,
    })
    .build()
    .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, user_env),
        other => panic!("expected MissingEnvVar for user_env, got {other:?}"),
    }
}

#[test]
fn test_digest_strategy_missing_pass_env_returns_missing_env_var() {
    let user_env = "SWE_AUTH_DIGEST_MISS_U_02";
    let pass_env = "SWE_AUTH_DIGEST_MISS_P_02";
    std::env::set_var(user_env, "alice");
    std::env::remove_var(pass_env);
    let err = Builder::with_config(AuthConfig::Digest {
        user_env: user_env.into(),
        password_env: pass_env.into(),
        realm: None,
    })
    .build()
    .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, pass_env),
        other => panic!("expected MissingEnvVar for password_env, got {other:?}"),
    }
    std::env::remove_var(user_env);
}

// ---------------------------------------------------------------------------
// Happy paths — both envs set
// ---------------------------------------------------------------------------

#[test]
fn test_digest_strategy_builds_when_both_envs_set_no_realm() {
    let user_env = "SWE_AUTH_DIGEST_OK_U_01";
    let pass_env = "SWE_AUTH_DIGEST_OK_P_01";
    std::env::set_var(user_env, "alice");
    std::env::set_var(pass_env, "s3cr3t");
    Builder::with_config(AuthConfig::Digest {
        user_env: user_env.into(),
        password_env: pass_env.into(),
        realm: None,
    })
    .build()
    .expect("Digest with both envs and no realm must build");
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
}

#[test]
fn test_digest_strategy_builds_when_both_envs_set_with_realm() {
    let user_env = "SWE_AUTH_DIGEST_OK_U_02";
    let pass_env = "SWE_AUTH_DIGEST_OK_P_02";
    std::env::set_var(user_env, "bob");
    std::env::set_var(pass_env, "password");
    Builder::with_config(AuthConfig::Digest {
        user_env: user_env.into(),
        password_env: pass_env.into(),
        realm: Some("api.example.com".into()),
    })
    .build()
    .expect("Digest with both envs and a realm must build");
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
}

// ---------------------------------------------------------------------------
// realm field — optional, doesn't affect build
// ---------------------------------------------------------------------------

#[test]
fn test_digest_strategy_realm_none_and_some_both_build() {
    let user_env = "SWE_AUTH_DIGEST_REALM_U_01";
    let pass_env = "SWE_AUTH_DIGEST_REALM_P_01";
    std::env::set_var(user_env, "user");
    std::env::set_var(pass_env, "pass");

    Builder::with_config(AuthConfig::Digest {
        user_env: user_env.into(),
        password_env: pass_env.into(),
        realm: None,
    })
    .build()
    .expect("Digest realm=None must build");

    Builder::with_config(AuthConfig::Digest {
        user_env: user_env.into(),
        password_env: pass_env.into(),
        realm: Some("realm.example".into()),
    })
    .build()
    .expect("Digest realm=Some(…) must build");

    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
}

// ---------------------------------------------------------------------------
// Middleware wiring
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_digest_strategy_middleware_wires_into_reqwest_middleware() {
    let user_env = "SWE_AUTH_DIGEST_WIRE_U_01";
    let pass_env = "SWE_AUTH_DIGEST_WIRE_P_01";
    std::env::set_var(user_env, "alice");
    std::env::set_var(pass_env, "wonderland");
    let mw = Builder::with_config(AuthConfig::Digest {
        user_env: user_env.into(),
        password_env: pass_env.into(),
        realm: None,
    })
    .build()
    .expect("Digest build ok");
    let _client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
        .with(mw)
        .build();
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
}

// ---------------------------------------------------------------------------
// Debug — must not leak credentials
// ---------------------------------------------------------------------------

#[test]
fn test_digest_strategy_middleware_debug_does_not_expose_password() {
    let user_env = "SWE_AUTH_DIGEST_DBG_U_01";
    let pass_env = "SWE_AUTH_DIGEST_DBG_P_01";
    let secret_pass = "DIGEST_SECRET_PASS_UNIQUE_MARKER";
    std::env::set_var(user_env, "dbg-user");
    std::env::set_var(pass_env, secret_pass);
    let mw = Builder::with_config(AuthConfig::Digest {
        user_env: user_env.into(),
        password_env: pass_env.into(),
        realm: None,
    })
    .build()
    .expect("build ok");
    let s = format!("{mw:?}");
    assert!(
        !s.contains(secret_pass),
        "AuthMiddleware Debug must not expose the digest password: {s}"
    );
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
}

// ---------------------------------------------------------------------------
// Send + Sync
// ---------------------------------------------------------------------------

#[test]
fn test_digest_strategy_auth_middleware_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<swe_edge_http_auth::AuthMiddleware>();
}
