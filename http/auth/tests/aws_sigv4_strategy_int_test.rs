//! Integration tests for the AwsSigV4 strategy path.
//!
//! The strategy itself is `pub(crate)`.  Observable effects through
//! `Builder::build()` and `AuthMiddleware`:
//! - Missing access_key_env → `Error::MissingEnvVar { name: access_key_env }`
//! - Missing secret_key_env (when access key is present) → MissingEnvVar
//! - Missing session_token_env (when declared) → MissingEnvVar
//! - All required envs set → build succeeds
//! - Session_token_env = None (not declared) → not required, build succeeds
//! - The built middleware's Debug output identifies the crate
//!
//! SigV4 header-correctness (Authorization: AWS4-HMAC-SHA256 …) is
//! covered by the core-unit tests inside `aws_sigv4_strategy.rs`.

use swe_http_auth::{AuthConfig, Builder, Error};

fn sigv4_config(ak: &str, sk: &str, st: Option<&str>) -> AuthConfig {
    AuthConfig::AwsSigV4 {
        access_key_env: ak.into(),
        secret_key_env: sk.into(),
        session_token_env: st.map(Into::into),
        region: "us-east-1".into(),
        service: "s3".into(),
    }
}

// ---------------------------------------------------------------------------
// Missing env vars at build time
// ---------------------------------------------------------------------------

#[test]
fn test_aws_sigv4_missing_access_key_env_returns_missing_env_var() {
    let ak_env = "SWE_AUTH_AWS_MISS_AK_01";
    let sk_env = "SWE_AUTH_AWS_MISS_SK_01";
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
    let err = Builder::with_config(sigv4_config(ak_env, sk_env, None))
        .build()
        .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, ak_env),
        other => panic!("expected MissingEnvVar for access key, got {other:?}"),
    }
}

#[test]
fn test_aws_sigv4_missing_secret_key_env_returns_missing_env_var() {
    let ak_env = "SWE_AUTH_AWS_MISS_AK_02";
    let sk_env = "SWE_AUTH_AWS_MISS_SK_02";
    // Only access key is present; secret key is absent.
    std::env::set_var(ak_env, "AKID");
    std::env::remove_var(sk_env);
    let err = Builder::with_config(sigv4_config(ak_env, sk_env, None))
        .build()
        .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, sk_env),
        other => panic!("expected MissingEnvVar for secret key, got {other:?}"),
    }
    std::env::remove_var(ak_env);
}

#[test]
fn test_aws_sigv4_missing_session_token_env_returns_missing_env_var() {
    let ak_env = "SWE_AUTH_AWS_MISS_AK_03";
    let sk_env = "SWE_AUTH_AWS_MISS_SK_03";
    let st_env = "SWE_AUTH_AWS_MISS_ST_03";
    std::env::set_var(ak_env, "AKID");
    std::env::set_var(sk_env, "SECRET");
    std::env::remove_var(st_env); // declared but absent
    let err = Builder::with_config(sigv4_config(ak_env, sk_env, Some(st_env)))
        .build()
        .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, st_env),
        other => panic!("expected MissingEnvVar for session token, got {other:?}"),
    }
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
}

// ---------------------------------------------------------------------------
// Happy paths
// ---------------------------------------------------------------------------

#[test]
fn test_aws_sigv4_builds_when_ak_and_sk_envs_are_set() {
    let ak_env = "SWE_AUTH_AWS_OK_AK_01";
    let sk_env = "SWE_AUTH_AWS_OK_SK_01";
    std::env::set_var(ak_env, "AKIATEST123456");
    std::env::set_var(sk_env, "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
    Builder::with_config(sigv4_config(ak_env, sk_env, None))
        .build()
        .expect("AwsSigV4 with ak+sk envs set must build");
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
}

#[test]
fn test_aws_sigv4_builds_when_all_three_envs_set() {
    let ak_env = "SWE_AUTH_AWS_OK_AK_02";
    let sk_env = "SWE_AUTH_AWS_OK_SK_02";
    let st_env = "SWE_AUTH_AWS_OK_ST_02";
    std::env::set_var(ak_env, "AKIATEST");
    std::env::set_var(sk_env, "SECRET");
    std::env::set_var(st_env, "SESSION_TOKEN");
    Builder::with_config(sigv4_config(ak_env, sk_env, Some(st_env)))
        .build()
        .expect("AwsSigV4 with all three envs set must build");
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
    std::env::remove_var(st_env);
}

#[test]
fn test_aws_sigv4_session_token_not_required_when_none() {
    // session_token_env = None means no session token is required or used.
    // The builder must succeed without any session-token env set.
    let ak_env = "SWE_AUTH_AWS_NOST_AK_01";
    let sk_env = "SWE_AUTH_AWS_NOST_SK_01";
    let st_env = "SWE_AUTH_AWS_NOST_ST_01"; // not declared in config
    std::env::set_var(ak_env, "AKIATEST");
    std::env::set_var(sk_env, "SECRET");
    std::env::remove_var(st_env); // absent but not declared → irrelevant
    Builder::with_config(AuthConfig::AwsSigV4 {
        access_key_env: ak_env.into(),
        secret_key_env: sk_env.into(),
        session_token_env: None, // not declared
        region: "eu-central-1".into(),
        service: "execute-api".into(),
    })
    .build()
    .expect("AwsSigV4 with session_token_env=None must build without that env var");
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
}

// ---------------------------------------------------------------------------
// Region + service are stored, not env-resolved
// ---------------------------------------------------------------------------

#[test]
fn test_aws_sigv4_region_and_service_are_stored_as_literals() {
    // region and service are plain strings, not env-var references —
    // they don't go through the resolver. Test by using unusual values
    // that would fail if accidentally treated as env var names.
    let ak_env = "SWE_AUTH_AWS_LIT_AK_01";
    let sk_env = "SWE_AUTH_AWS_LIT_SK_01";
    std::env::set_var(ak_env, "AKID");
    std::env::set_var(sk_env, "SECRET");
    Builder::with_config(AuthConfig::AwsSigV4 {
        access_key_env: ak_env.into(),
        secret_key_env: sk_env.into(),
        session_token_env: None,
        region: "ap-southeast-1".into(),   // literal, not an env var
        service: "sts".into(),             // literal
    })
    .build()
    .expect("region/service are literals — must not affect build success");
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
}

// ---------------------------------------------------------------------------
// Debug — does not leak credentials
// ---------------------------------------------------------------------------

#[test]
fn test_aws_sigv4_auth_middleware_debug_does_not_leak_credentials() {
    let ak_env = "SWE_AUTH_AWS_DBG_AK_01";
    let sk_env = "SWE_AUTH_AWS_DBG_SK_01";
    let secret_marker = "SUPER_SECRET_KEY_DO_NOT_PRINT_XYZ";
    std::env::set_var(ak_env, "AKIATESTMARKER");
    std::env::set_var(sk_env, secret_marker);
    let mw = Builder::with_config(AuthConfig::AwsSigV4 {
        access_key_env: ak_env.into(),
        secret_key_env: sk_env.into(),
        session_token_env: None,
        region: "us-east-1".into(),
        service: "s3".into(),
    })
    .build()
    .expect("build ok");
    let s = format!("{mw:?}");
    assert!(
        !s.contains(secret_marker),
        "AuthMiddleware Debug must not expose the secret key value: {s}"
    );
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
}
