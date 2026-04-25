//! End-to-end tests for the swe_edge_egress SAF builder surface.

use swe_edge_egress::{
    build_memory_database, memory_database, Builder, DatabaseRead, DatabaseWrite, Record,
};

/// @covers: memory_database
#[tokio::test]
async fn test_memory_database_insert_and_get_round_trips() {
    let db = memory_database();
    let rec = Record::new().set("id", serde_json::Value::String("hello".into()));
    db.insert("t", rec).await.unwrap();
    let found = db.get_by_id("t", "hello").await.unwrap();
    assert!(found.is_some());
}

#[test]
fn test_builder_new_constructs_successfully() {
    let _ = Builder::new();
}

#[tokio::test]
async fn test_build_memory_database_returns_usable_gateway() {
    let db = build_memory_database();
    let rec = Record::new().set("id", serde_json::Value::String("k".into()));
    db.insert("t", rec).await.unwrap();
    let found = db.get_by_id("t", "k").await.unwrap();
    assert!(found.is_some());
}
