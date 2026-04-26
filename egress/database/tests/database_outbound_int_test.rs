//! Integration tests for the database outbound domain.

use swe_edge_egress_database::{
    memory_database_impl, DatabaseRead, DatabaseWrite, Record,
};
use serde_json::Value;

/// @covers: memory_database_impl — insert and read round-trip via port trait.
#[tokio::test]
async fn test_memory_database_insert_and_get_round_trip() {
    let db = memory_database_impl();
    let rec = Record::new().set("id", Value::String("test-id".into()));
    db.insert("users", rec).await.unwrap();
    let found = db.get_by_id("users", "test-id").await.unwrap();
    assert!(found.is_some(), "expected record to be found after insert");
    assert_eq!(
        found.unwrap().get("id"),
        Some(&Value::String("test-id".into()))
    );
}

/// @covers: DatabaseRead::count — returns zero for empty table.
#[tokio::test]
async fn test_memory_database_count_empty_table_returns_zero() {
    let db = memory_database_impl();
    let count = db.count("nonexistent").await.unwrap();
    assert_eq!(count, 0);
}

/// @covers: DatabaseWrite::delete — removing a record decrements count.
#[tokio::test]
async fn test_memory_database_delete_decrements_count() {
    let db = memory_database_impl();
    let rec = Record::new().set("id", Value::String("del-1".into()));
    db.insert("items", rec).await.unwrap();
    assert_eq!(db.count("items").await.unwrap(), 1);
    db.delete("items", "del-1").await.unwrap();
    assert_eq!(db.count("items").await.unwrap(), 0);
}
