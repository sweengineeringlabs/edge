//! End-to-end tests for the swe_edge_egress SAF builder surface.

use swe_edge_egress::{memory_database, Builder, DatabaseRead, DatabaseWrite};

/// @covers: memory_database
#[test]
fn test_memory_database_put_and_get_round_trips() {
    let db = memory_database();
    db.put("hello", b"world").unwrap();
    assert_eq!(db.get("hello").unwrap(), Some(b"world".to_vec()));
}

/// @covers: Builder::new
#[test]
fn test_builder_new_constructs_successfully() {
    let _ = Builder::new();
}

/// @covers: Builder::build_memory_database
#[test]
fn test_build_memory_database_returns_usable_gateway() {
    let db = Builder::new().build_memory_database();
    db.put("k", b"v").unwrap();
    assert_eq!(db.get("k").unwrap(), Some(b"v".to_vec()));
}
