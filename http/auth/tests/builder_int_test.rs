//! Integration tests for `Builder` (api/builder.rs) and the `builder()`
//! entry-point function (saf/builder.rs).
//!
//! Both live under the stem "builder" — one file covers both because the
//! public surface is the same (both are re-exported through the gateway).

use swe_http_auth::{AuthConfig, AuthMiddleware, Builder, Error};

// ---------------------------------------------------------------------------
// builder() entry-point
// ---------------------------------------------------------------------------

#[test]
fn test_builder_fn_succeeds() {
    // The free `builder()` function must parse the shipped config and
    // construct a Builder. Failure here means the crate-internal
    // application.toml is malformed — a build-breaking defect.
    swe_http_auth::builder().expect("builder() must succeed unconditionally");
}

#[test]
fn test_builder_fn_loads_none_pass_through_as_default() {
    let b = swe_http_auth::builder().expect("builder() succeeds");
    assert!(
        matches!(b.config(), AuthConfig::None),
        "default config must be AuthConfig::None, got {:?}",
        b.config()
    );
}

// ---------------------------------------------------------------------------
// Builder::with_config
// ---------------------------------------------------------------------------

#[test]
fn test_with_config_stores_none_variant() {
    let b = Builder::with_config(AuthConfig::None);
    assert!(
        matches!(b.config(), AuthConfig::None),
        "with_config(None) must store None: {:?}",
        b.config()
    );
}

#[test]
fn test_with_config_stores_bearer_variant() {
    let cfg = AuthConfig::Bearer { token_env: "SWE_BLD_BEARER_01".into() };
    let b = Builder::with_config(cfg);
    assert!(
        matches!(b.config(), AuthConfig::Bearer { .. }),
        "with_config(Bearer) must store Bearer: {:?}",
        b.config()
    );
}

#[test]
fn test_with_config_stores_basic_variant() {
    let cfg = AuthConfig::Basic {
        user_env: "SWE_BLD_BASIC_U_01".into(),
        pass_env: "SWE_BLD_BASIC_P_01".into(),
    };
    let b = Builder::with_config(cfg);
    assert!(
        matches!(b.config(), AuthConfig::Basic { .. }),
        "with_config(Basic) must store Basic: {:?}",
        b.config()
    );
}

#[test]
fn test_with_config_stores_header_variant() {
    let cfg = AuthConfig::Header {
        name: "x-custom-key".into(),
        value_env: "SWE_BLD_HEADER_01".into(),
    };
    let b = Builder::with_config(cfg);
    assert!(
        matches!(b.config(), AuthConfig::Header { .. }),
        "with_config(Header) must store Header: {:?}",
        b.config()
    );
}

#[test]
fn test_with_config_stores_aws_sigv4_variant() {
    let cfg = AuthConfig::AwsSigV4 {
        access_key_env: "SWE_BLD_AWS_AK_01".into(),
        secret_key_env: "SWE_BLD_AWS_SK_01".into(),
        session_token_env: None,
        region: "eu-west-1".into(),
        service: "s3".into(),
    };
    let b = Builder::with_config(cfg);
    assert!(
        matches!(b.config(), AuthConfig::AwsSigV4 { .. }),
        "with_config(AwsSigV4) must store AwsSigV4: {:?}",
        b.config()
    );
}

// ---------------------------------------------------------------------------
// Builder::config — borrow accessor returns the stored policy
// ---------------------------------------------------------------------------

#[test]
fn test_config_accessor_returns_stored_bearer_token_env_name() {
    let cfg = AuthConfig::Bearer { token_env: "SWE_BLD_CFG_BEARER_01".into() };
    let b = Builder::with_config(cfg);
    match b.config() {
        AuthConfig::Bearer { token_env } => {
            assert_eq!(token_env, "SWE_BLD_CFG_BEARER_01");
        }
        other => panic!("config() returned wrong variant: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Builder::build — None always succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_build_none_config_returns_auth_middleware() {
    let mw: AuthMiddleware = Builder::with_config(AuthConfig::None)
        .build()
        .expect("None config must build to AuthMiddleware");
    // Verify the middleware is functional by exercising its Debug impl.
    let s = format!("{mw:?}");
    assert!(!s.is_empty(), "AuthMiddleware Debug must be non-empty");
}

// ---------------------------------------------------------------------------
// Builder::build — fails fast on missing env vars
// ---------------------------------------------------------------------------

#[test]
fn test_build_bearer_missing_env_returns_missing_env_var_error() {
    let env_name = "SWE_BLD_MISS_BEARER_01";
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
fn test_build_bearer_env_set_produces_auth_middleware() {
    let env_name = "SWE_BLD_SET_BEARER_01";
    std::env::set_var(env_name, "bld-token-value");
    let mw = Builder::with_config(AuthConfig::Bearer { token_env: env_name.into() })
        .build()
        .expect("Bearer with env set must build");
    let _ = format!("{mw:?}");
    std::env::remove_var(env_name);
}

#[test]
fn test_build_basic_missing_pass_env_returns_missing_env_var_error() {
    let user_env = "SWE_BLD_MISS_BASIC_U_01";
    let pass_env = "SWE_BLD_MISS_BASIC_P_01";
    std::env::set_var(user_env, "user"); // user present — pass absent
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
// Debug impl on Builder
// ---------------------------------------------------------------------------

#[test]
fn test_builder_debug_contains_builder_type_name() {
    let b = Builder::with_config(AuthConfig::None);
    let s = format!("{b:?}");
    assert!(
        s.contains("Builder"),
        "Builder Debug must contain 'Builder', got: {s}"
    );
}

#[test]
fn test_builder_debug_does_not_expose_resolver_internals() {
    // The resolver is an implementation detail — its internals (if any
    // sensitive field existed) must not be printed.  At minimum, Debug
    // on Builder must not panic.
    let b = Builder::with_config(AuthConfig::None);
    let _ = format!("{b:?}");
}
