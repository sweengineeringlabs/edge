//! Integration tests for gateway.

use gateway::*;

/// @covers: run
#[test]
fn test_run_succeeds() {
    assert!(run().is_ok());
}

/// @covers: execute
#[test]
fn test_execute_with_default_config() {
    let config = Config::default();
    assert!(execute(&config).is_ok());
}
