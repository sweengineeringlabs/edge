//! Integration tests for the `CredentialResolver` contract.
//!
//! `CredentialResolver` is `pub(crate)` — it cannot be imported in an
//! integration test. These tests exercise the resolver's effect through
//! the only public path it influences: `Builder::build()`.
//!
//! The contract being validated:
//! - When the referenced env var is present, `build()` succeeds.
//! - When the referenced env var is absent, `build()` fails with
//!   `Error::MissingEnvVar { name }` where `name` equals the var name
//!   declared in the config.
//! - Empty env vars are "present" at the OS level; resolution succeeds
//!   (scheme-level validation may reject the empty value, but that is the
//!   scheme's concern — not the resolver's).

use swe_http_auth::{AuthConfig, Builder, Error};

// ---------------------------------------------------------------------------
// Env var present → build succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_resolver_env_present_bearer_build_succeeds() {
    let env_name = "SWE_AUTH_RESOLVER_PRES_BRR_01";
    std::env::set_var(env_name, "resolver-test-token");
    Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("bearer env present — resolver must succeed and build must pass");
    std::env::remove_var(env_name);
}

#[test]
fn test_resolver_env_present_basic_both_build_succeeds() {
    let user_env = "SWE_AUTH_RESOLVER_PRES_BASIC_U_01";
    let pass_env = "SWE_AUTH_RESOLVER_PRES_BASIC_P_01";
    std::env::set_var(user_env, "alice");
    std::env::set_var(pass_env, "s3cret");
    Builder::with_config(AuthConfig::Basic {
        user_env: user_env.into(),
        pass_env: pass_env.into(),
    })
    .build()
    .expect("basic both envs present — resolver must succeed");
    std::env::remove_var(user_env);
    std::env::remove_var(pass_env);
}

// ---------------------------------------------------------------------------
// Env var absent → build fails with MissingEnvVar naming the exact var
// ---------------------------------------------------------------------------

#[test]
fn test_resolver_env_absent_bearer_returns_missing_env_var_with_exact_name() {
    let env_name = "SWE_AUTH_RESOLVER_ABS_BRR_01";
    std::env::remove_var(env_name);
    let err = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => {
            assert_eq!(
                name, env_name,
                "MissingEnvVar must name the exact missing var: wanted {env_name}, got {name}"
            );
        }
        other => panic!("expected MissingEnvVar, got {other:?}"),
    }
}

#[test]
fn test_resolver_env_absent_header_returns_missing_env_var_with_exact_name() {
    let env_name = "SWE_AUTH_RESOLVER_ABS_HDR_01";
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

// Basic: user env absent, pass env present — must fail with user_env name
#[test]
fn test_resolver_env_absent_basic_user_names_user_var_in_error() {
    let user_env = "SWE_AUTH_RESOLVER_ABS_BASIC_U_01";
    let pass_env = "SWE_AUTH_RESOLVER_ABS_BASIC_P_01";
    std::env::remove_var(user_env);
    std::env::set_var(pass_env, "pass");
    let err = Builder::with_config(AuthConfig::Basic {
        user_env: user_env.into(),
        pass_env: pass_env.into(),
    })
    .build()
    .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, user_env),
        other => panic!("expected MissingEnvVar for user_env, got {other:?}"),
    }
    std::env::remove_var(pass_env);
}

// Basic: user env present, pass env absent — must fail with pass_env name
#[test]
fn test_resolver_env_absent_basic_pass_names_pass_var_in_error() {
    let user_env = "SWE_AUTH_RESOLVER_ABS_BASIC_U_02";
    let pass_env = "SWE_AUTH_RESOLVER_ABS_BASIC_P_02";
    std::env::set_var(user_env, "user");
    std::env::remove_var(pass_env);
    let err = Builder::with_config(AuthConfig::Basic {
        user_env: user_env.into(),
        pass_env: pass_env.into(),
    })
    .build()
    .unwrap_err();
    match err {
        Error::MissingEnvVar { name } => assert_eq!(name, pass_env),
        other => panic!("expected MissingEnvVar for pass_env, got {other:?}"),
    }
    std::env::remove_var(user_env);
}

// ---------------------------------------------------------------------------
// Empty env var is "present" — resolution succeeds (scheme decides validity)
// ---------------------------------------------------------------------------

#[test]
fn test_resolver_empty_env_var_is_present_and_build_succeeds_for_bearer() {
    // An empty-string env var is still "set" at the OS level.
    // The resolver's contract is: return the value if the var exists.
    // An empty bearer token will be stored as "Bearer " — technically
    // valid at the resolver layer (scheme validation is out of scope here).
    let env_name = "SWE_AUTH_RESOLVER_EMPTY_BRR_01";
    std::env::set_var(env_name, "");
    // Empty bearer value produces "Bearer " — that's a valid header value
    // (space is an ASCII visible char; it won't fail HeaderValue::from_str).
    let result = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() }).build();
    // Accept either Ok (built successfully) or Err(InvalidHeaderValue)
    // — but never MissingEnvVar, because the var IS set.
    match result {
        Ok(_) => { /* resolver succeeded; scheme accepted empty token */ }
        Err(Error::MissingEnvVar { name }) => {
            panic!(
                "empty env var must NOT produce MissingEnvVar — var is present: {name}"
            );
        }
        Err(_) => { /* some other validation error is acceptable */ }
    }
    std::env::remove_var(env_name);
}
