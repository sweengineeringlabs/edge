//! Integration tests for `CredentialSource` behaviour.
//!
//! `CredentialSource` is `pub(crate)` — it cannot be imported here.
//! These tests exercise its effects through `Builder::build()`:
//! the `EnvVar` source kind is the only one that exists, and its
//! resolution failure message must identify the var name declared
//! in the config.
//!
//! Specifically:
//! - Each `AuthConfig` field that references an env var (token_env,
//!   user_env, pass_env, value_env, etc.) is wrapped in a
//!   `CredentialSource::EnvVar` internally.  Missing env → error
//!   carrying the exact name.
//! - The config stores only the env-var NAME, never a resolved value.

use swe_edge_egress_auth::{AuthConfig, Builder, Error};

// ---------------------------------------------------------------------------
// Bearer: source is the token_env name
// ---------------------------------------------------------------------------

#[test]
fn test_credential_source_bearer_identifies_missing_token_env_by_name() {
    let env_name = "SWE_AUTH_SRC_BRR_01";
    std::env::remove_var(env_name);
    let err = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(
            name, env_name,
            "MissingEnvVar must carry the token_env name: {name}"
        ),
        other => panic!("expected MissingEnvVar, got {other:?}"),
    }
}

#[test]
fn test_credential_source_bearer_resolved_when_token_env_is_present() {
    let env_name = "SWE_AUTH_SRC_BRR_02";
    std::env::set_var(env_name, "src-bearer-value");
    Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("token_env present — credential source must resolve");
    std::env::remove_var(env_name);
}

// ---------------------------------------------------------------------------
// Basic: two sources (user_env, pass_env)
// ---------------------------------------------------------------------------

#[test]
fn test_credential_source_basic_identifies_missing_pass_env_by_name() {
    let user_env = "SWE_AUTH_SRC_BASIC_U_01";
    let pass_env = "SWE_AUTH_SRC_BASIC_P_01";
    std::env::set_var(user_env, "user-present");
    std::env::remove_var(pass_env); // Only pass is missing
    let err = Builder::with_config(AuthConfig::Basic {
        user_env: user_env.into(),
        pass_env: pass_env.into(),
    })
    .build()
    .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(
            name, pass_env,
            "MissingEnvVar must carry the pass_env name: {name}"
        ),
        other => panic!("expected MissingEnvVar, got {other:?}"),
    }
    std::env::remove_var(user_env);
}

#[test]
fn test_credential_source_basic_both_resolved_when_both_envs_present() {
    let user_env = "SWE_AUTH_SRC_BASIC_U_02";
    let pass_env = "SWE_AUTH_SRC_BASIC_P_02";
    std::env::set_var(user_env, "src-user");
    std::env::set_var(pass_env, "src-pass");
    Builder::with_config(AuthConfig::Basic {
        user_env: user_env.into(),
        pass_env: pass_env.into(),
    })
    .build()
    .expect("both envs present — both sources must resolve");
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
}

// ---------------------------------------------------------------------------
// Header: source is the value_env name
// ---------------------------------------------------------------------------

#[test]
fn test_credential_source_header_identifies_missing_value_env_by_name() {
    let env_name = "SWE_AUTH_SRC_HDR_01";
    std::env::remove_var(env_name);
    let err = Builder::with_config(AuthConfig::Header {
        name: "x-test-key".into(),
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
// Digest: two sources (user_env, password_env)
// ---------------------------------------------------------------------------

#[test]
fn test_credential_source_digest_identifies_missing_user_env_by_name() {
    let user_env = "SWE_AUTH_SRC_DIG_U_01";
    let pass_env = "SWE_AUTH_SRC_DIG_P_01";
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

// ---------------------------------------------------------------------------
// AwsSigV4: two required sources + one optional (session_token_env)
// ---------------------------------------------------------------------------

#[test]
fn test_credential_source_aws_sigv4_identifies_missing_access_key_by_name() {
    let ak_env = "SWE_AUTH_SRC_AWS_AK_01";
    let sk_env = "SWE_AUTH_SRC_AWS_SK_01";
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
    let err = Builder::with_config(AuthConfig::AwsSigV4 {
        access_key_env: ak_env.into(),
        secret_key_env: sk_env.into(),
        session_token_env: None,
        region: "us-east-1".into(),
        service: "sts".into(),
    })
    .build()
    .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, ak_env),
        other => panic!("expected MissingEnvVar for access_key_env, got {other:?}"),
    }
}

#[test]
fn test_credential_source_aws_sigv4_optional_session_token_not_required() {
    // When session_token_env is None, the optional source is skipped — no
    // MissingEnvVar should fire for it.
    let ak_env = "SWE_AUTH_SRC_AWS_AK_02";
    let sk_env = "SWE_AUTH_SRC_AWS_SK_02";
    std::env::set_var(ak_env, "AKID_src_test");
    std::env::set_var(sk_env, "SECRET_src_test");
    Builder::with_config(AuthConfig::AwsSigV4 {
        access_key_env: ak_env.into(),
        secret_key_env: sk_env.into(),
        session_token_env: None, // optional source not declared → not resolved
        region: "ap-east-1".into(),
        service: "s3".into(),
    })
    .build()
    .expect("AwsSigV4 with no session token must build when ak/sk envs are set");
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
}

#[test]
fn test_credential_source_aws_sigv4_session_token_env_resolved_when_present() {
    let ak_env = "SWE_AUTH_SRC_AWS_AK_03";
    let sk_env = "SWE_AUTH_SRC_AWS_SK_03";
    let st_env = "SWE_AUTH_SRC_AWS_ST_03";
    std::env::set_var(ak_env, "AKID_src_st");
    std::env::set_var(sk_env, "SECRET_src_st");
    std::env::set_var(st_env, "SESSION_src_st");
    Builder::with_config(AuthConfig::AwsSigV4 {
        access_key_env: ak_env.into(),
        secret_key_env: sk_env.into(),
        session_token_env: Some(st_env.into()),
        region: "us-east-1".into(),
        service: "s3".into(),
    })
    .build()
    .expect("AwsSigV4 with all three envs set must build");
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
    std::env::remove_var(st_env);
}

#[test]
fn test_credential_source_aws_sigv4_session_token_env_absent_fails() {
    let ak_env = "SWE_AUTH_SRC_AWS_AK_04";
    let sk_env = "SWE_AUTH_SRC_AWS_SK_04";
    let st_env = "SWE_AUTH_SRC_AWS_ST_04";
    std::env::set_var(ak_env, "AKID");
    std::env::set_var(sk_env, "SECRET");
    std::env::remove_var(st_env); // declared but not set
    let err = Builder::with_config(AuthConfig::AwsSigV4 {
        access_key_env: ak_env.into(),
        secret_key_env: sk_env.into(),
        session_token_env: Some(st_env.into()),
        region: "us-east-1".into(),
        service: "s3".into(),
    })
    .build()
    .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, st_env),
        other => panic!("expected MissingEnvVar for session_token_env, got {other:?}"),
    }
    std::env::remove_var(ak_env);
    std::env::remove_var(sk_env);
}
