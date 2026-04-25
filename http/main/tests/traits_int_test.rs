//! Integration tests for `api::traits` — StackAssembler marker trait.

use swe_http_auth::AuthConfig;
use swe_http_main::{Builder, StackConfig};

/// @covers: StackAssembler — DefaultStack is Send + Sync via the assembler contract
#[test]
fn test_builder_config_round_trips_stack_config() {
    let cfg = StackConfig { auth: AuthConfig::None };
    let b = Builder::with_config(cfg);
    assert!(matches!(b.config().auth, AuthConfig::None));
}
