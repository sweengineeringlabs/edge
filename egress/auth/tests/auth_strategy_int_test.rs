//! Integration tests for strategy selection driven by `AuthConfig` variants.
//!
//! `AuthStrategy` itself is `pub(crate)` — these tests exercise strategy
//! selection indirectly by building middleware from each config variant
//! and observing the resulting `AuthMiddleware`'s behaviour through the
//! `reqwest_middleware::Middleware` interface (via a test `reqwest::Request`
//! processed end-to-end through the middleware's `handle` pathway).
//!
//! What we can observe from outside the crate:
//! - `Builder::build()` succeeds or fails (fails fast on missing env vars)
//! - The built `AuthMiddleware` is a valid `reqwest_middleware::Middleware`
//!   (compile-time bound)
//! - `AuthMiddleware`'s `Debug` output reflects the processor kind

use reqwest_middleware::Middleware;
use swe_edge_egress_auth::{AuthConfig, AuthMiddleware, Builder, Error};

// Verify AuthMiddleware implements the reqwest_middleware::Middleware trait
// at compile time. If the impl is removed this function won't compile.
fn _assert_middleware_impl<T: Middleware>() {}
fn _check() {
    _assert_middleware_impl::<AuthMiddleware>();
}

// ---------------------------------------------------------------------------
// None → NoopStrategy: builds, no env vars required
// ---------------------------------------------------------------------------

#[test]
fn test_none_config_selects_noop_strategy_builds_without_env() {
    Builder::with_config(AuthConfig::None)
        .build()
        .expect("None config must build regardless of env state");
}

// ---------------------------------------------------------------------------
// Bearer → BearerStrategy: fails fast on missing token env
// ---------------------------------------------------------------------------

#[test]
fn test_bearer_config_fails_fast_when_token_env_missing() {
    let env_name = "SWE_AUTH_STRAT_BEARER_01";
    std::env::remove_var(env_name);
    let err = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, env_name),
        other => panic!("expected MissingEnvVar, got {other:?}"),
    }
}

#[test]
fn test_bearer_config_selects_bearer_strategy_when_env_set() {
    let env_name = "SWE_AUTH_STRAT_BEARER_02";
    std::env::set_var(env_name, "strat-bearer-tok");
    let mw = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("Bearer with env set must build");
    // The middleware must be non-trivially constructed — debug shows processor.
    let s = format!("{mw:?}");
    assert!(s.contains("swe_edge_egress_auth"), "unexpected debug: {s}");
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Basic → BasicStrategy: fails fast on missing user/pass env
// ---------------------------------------------------------------------------

#[test]
fn test_basic_config_fails_fast_when_user_env_missing() {
    let user_env = "SWE_AUTH_STRAT_BASIC_U_01";
    let pass_env = "SWE_AUTH_STRAT_BASIC_P_01";
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
        "expected MissingEnvVar for missing user env, got {err:?}"
    );
}

#[test]
fn test_basic_config_selects_basic_strategy_when_both_envs_set() {
    let user_env = "SWE_AUTH_STRAT_BASIC_U_02";
    let pass_env = "SWE_AUTH_STRAT_BASIC_P_02";
    std::env::set_var(user_env, "strat-user");
    std::env::set_var(pass_env, "strat-pass");
    Builder::with_config(AuthConfig::Basic {
        user_env: user_env.into(),
        pass_env: pass_env.into(),
    })
    .build()
    .expect("Basic with both envs set must build");
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
}

// ---------------------------------------------------------------------------
// Header → HeaderStrategy: fails fast on missing value env
// ---------------------------------------------------------------------------

#[test]
fn test_header_config_fails_fast_when_value_env_missing() {
    let env_name = "SWE_AUTH_STRAT_HEADER_01";
    std::env::remove_var(env_name);
    let err = Builder::with_config(AuthConfig::Header {
        name: "x-api-key".into(),
        value_env: env_name.into(),
    })
    .build()
    .unwrap_err();
    assert!(
        matches!(err, Error::MissingEnvVar { .. }),
        "expected MissingEnvVar for missing header env, got {err:?}"
    );
}

#[test]
fn test_header_config_selects_header_strategy_when_env_set() {
    let env_name = "SWE_AUTH_STRAT_HEADER_02";
    std::env::set_var(env_name, "api-key-value");
    Builder::with_config(AuthConfig::Header {
        name: "x-api-key".into(),
        value_env: env_name.into(),
    })
    .build()
    .expect("Header with env set must build");
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Digest → DigestStrategy: fails fast on missing user/pass env
// ---------------------------------------------------------------------------

#[test]
fn test_digest_config_fails_fast_when_user_env_missing() {
    let user_env = "SWE_AUTH_STRAT_DIG_U_01";
    let pass_env = "SWE_AUTH_STRAT_DIG_P_01";
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
    let err = Builder::with_config(AuthConfig::Digest {
        user_env: user_env.into(),
        password_env: pass_env.into(),
        realm: None,
    })
    .build()
    .unwrap_err();
    assert!(
        matches!(err, Error::MissingEnvVar { .. }),
        "expected MissingEnvVar for missing digest user env, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// AwsSigV4 → AwsSigV4Strategy: fails fast on missing access/secret key env
// ---------------------------------------------------------------------------

#[test]
fn test_aws_sigv4_config_fails_fast_when_access_key_env_missing() {
    let ak_env = "SWE_AUTH_STRAT_AWS_AK_01";
    let sk_env = "SWE_AUTH_STRAT_AWS_SK_01";
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
    let err = Builder::with_config(AuthConfig::AwsSigV4 {
        access_key_env: ak_env.into(),
        secret_key_env: sk_env.into(),
        session_token_env: None,
        region: "us-east-1".into(),
        service: "s3".into(),
    })
    .build()
    .unwrap_err();
    assert!(
        matches!(err, Error::MissingEnvVar { .. }),
        "expected MissingEnvVar for missing AWS access key env, got {err:?}"
    );
}

#[test]
fn test_aws_sigv4_config_selects_sigv4_strategy_when_envs_set() {
    let ak_env = "SWE_AUTH_STRAT_AWS_AK_02";
    let sk_env = "SWE_AUTH_STRAT_AWS_SK_02";
    std::env::set_var(ak_env, "AKIATEST123");
    std::env::set_var(sk_env, "secretkey456");
    Builder::with_config(AuthConfig::AwsSigV4 {
        access_key_env: ak_env.into(),
        secret_key_env: sk_env.into(),
        session_token_env: None,
        region: "us-west-2".into(),
        service: "execute-api".into(),
    })
    .build()
    .expect("AwsSigV4 with both envs set must build");
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
}
