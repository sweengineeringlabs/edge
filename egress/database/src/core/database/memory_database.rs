//! In-memory database egress adapter — for testing and development.

use std::collections::HashMap;
use std::sync::Arc;

use futures::future::BoxFuture;
use parking_lot::RwLock;

use crate::api::aggregate::Record;
use crate::api::port::{DatabaseGateway, DatabaseRead, DatabaseWrite};
use crate::api::port::database_read::{DatabaseError, DatabaseResult, DbHealthCheck};
use crate::api::value_object::{QueryParams, WriteResult};

type Table = HashMap<String, Record>;

/// Thread-safe in-memory table store keyed by table name then record id.
pub(crate) struct MemoryDatabase {
    tables: Arc<RwLock<HashMap<String, Table>>>,
}

impl MemoryDatabase {
    pub(crate) fn new() -> Self {
        Self { tables: Arc::new(RwLock::new(HashMap::new())) }
    }

    fn next_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

impl DatabaseRead for MemoryDatabase {
    fn query(&self, params: QueryParams) -> BoxFuture<'_, DatabaseResult<Vec<Record>>> {
        let tables = Arc::clone(&self.tables);
        Box::pin(async move {
            let guard = tables.read();
            let table = match guard.get(&params.table) {
                Some(t) => t,
                None => return Ok(vec![]),
            };
            let mut results: Vec<Record> = table.values()
                .filter(|r| {
                    params.filters.iter().all(|(k, v)| {
                        r.fields.get(k) == Some(v)
                    })
                })
                .cloned()
                .collect();
            if let Some(limit) = params.limit {
                let offset = params.offset.unwrap_or(0);
                results = results.into_iter().skip(offset).take(limit).collect();
            }
            Ok(results)
        })
    }

    fn get_by_id(&self, table: &str, id: &str) -> BoxFuture<'_, DatabaseResult<Option<Record>>> {
        let tables = Arc::clone(&self.tables);
        let table = table.to_string();
        let id = id.to_string();
        Box::pin(async move {
            let guard = tables.read();
            Ok(guard.get(&table).and_then(|t| t.get(&id)).cloned())
        })
    }

    fn exists(&self, table: &str, id: &str) -> BoxFuture<'_, DatabaseResult<bool>> {
        let tables = Arc::clone(&self.tables);
        let table = table.to_string();
        let id = id.to_string();
        Box::pin(async move {
            let guard = tables.read();
            Ok(guard.get(&table).is_some_and(|t| t.contains_key(&id)))
        })
    }

    fn count(&self, table: &str) -> BoxFuture<'_, DatabaseResult<u64>> {
        let tables = Arc::clone(&self.tables);
        let table = table.to_string();
        Box::pin(async move {
            let guard = tables.read();
            Ok(guard.get(&table).map_or(0, |t| t.len() as u64))
        })
    }

    fn health_check(&self) -> BoxFuture<'_, DatabaseResult<DbHealthCheck>> {
        Box::pin(async move { Ok(DbHealthCheck::healthy()) })
    }
}

impl DatabaseWrite for MemoryDatabase {
    fn insert(&self, table: &str, record: Record) -> BoxFuture<'_, DatabaseResult<WriteResult>> {
        let tables = Arc::clone(&self.tables);
        let table = table.to_string();
        Box::pin(async move {
            let id = record.fields.get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(MemoryDatabase::next_id);
            let mut guard = tables.write();
            let t = guard.entry(table).or_default();
            if t.contains_key(&id) {
                return Err(DatabaseError::AlreadyExists(format!("id={id}")));
            }
            t.insert(id.clone(), record);
            Ok(WriteResult::inserted(id))
        })
    }

    fn update(&self, table: &str, id: &str, record: Record) -> BoxFuture<'_, DatabaseResult<WriteResult>> {
        let tables = Arc::clone(&self.tables);
        let table = table.to_string();
        let id = id.to_string();
        Box::pin(async move {
            let mut guard = tables.write();
            let t = guard.entry(table).or_default();
            if !t.contains_key(&id) {
                return Err(DatabaseError::NotFound(format!("id={id}")));
            }
            t.insert(id, record);
            Ok(WriteResult::rows(1))
        })
    }

    fn delete(&self, table: &str, id: &str) -> BoxFuture<'_, DatabaseResult<WriteResult>> {
        let tables = Arc::clone(&self.tables);
        let table = table.to_string();
        let id = id.to_string();
        Box::pin(async move {
            let mut guard = tables.write();
            let rows = guard.get_mut(&table).and_then(|t| t.remove(&id)).map_or(0, |_| 1);
            Ok(WriteResult::rows(rows))
        })
    }

    fn batch_insert(&self, table: &str, records: Vec<Record>) -> BoxFuture<'_, DatabaseResult<WriteResult>> {
        let tables = Arc::clone(&self.tables);
        let table = table.to_string();
        Box::pin(async move {
            let count = records.len() as u64;
            let mut guard = tables.write();
            let t = guard.entry(table).or_default();
            for record in records {
                let id = record.fields.get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(MemoryDatabase::next_id);
                t.insert(id, record);
            }
            Ok(WriteResult::rows(count))
        })
    }

    fn update_where(&self, table: &str, params: QueryParams, record: Record) -> BoxFuture<'_, DatabaseResult<WriteResult>> {
        let tables = Arc::clone(&self.tables);
        let table = table.to_string();
        Box::pin(async move {
            let mut guard = tables.write();
            let t = match guard.get_mut(&table) {
                Some(t) => t,
                None => return Ok(WriteResult::rows(0)),
            };
            let mut count = 0u64;
            for existing in t.values_mut() {
                if params.filters.iter().all(|(k, v)| existing.fields.get(k) == Some(v)) {
                    for (k, v) in &record.fields {
                        existing.fields.insert(k.clone(), v.clone());
                    }
                    count += 1;
                }
            }
            Ok(WriteResult::rows(count))
        })
    }

    fn delete_where(&self, table: &str, params: QueryParams) -> BoxFuture<'_, DatabaseResult<WriteResult>> {
        let tables = Arc::clone(&self.tables);
        let table = table.to_string();
        Box::pin(async move {
            let mut guard = tables.write();
            let t = match guard.get_mut(&table) {
                Some(t) => t,
                None => return Ok(WriteResult::rows(0)),
            };
            let to_remove: Vec<String> = t.iter()
                .filter(|(_, r)| params.filters.iter().all(|(k, v)| r.fields.get(k) == Some(v)))
                .map(|(id, _)| id.clone())
                .collect();
            let count = to_remove.len() as u64;
            for id in to_remove { t.remove(&id); }
            Ok(WriteResult::rows(count))
        })
    }
}

impl DatabaseGateway for MemoryDatabase {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn db() -> MemoryDatabase { MemoryDatabase::new() }

    fn rec(id: &str) -> Record {
        Record::new().set("id", Value::String(id.into()))
    }

    #[tokio::test]
    async fn test_insert_then_get_by_id_returns_record() {
        let d = db();
        d.insert("users", rec("u1")).await.unwrap();
        let r = d.get_by_id("users", "u1").await.unwrap();
        assert!(r.is_some());
        assert_eq!(r.unwrap().get("id"), Some(&Value::String("u1".into())));
    }

    #[tokio::test]
    async fn test_insert_duplicate_id_returns_already_exists_error() {
        let d = db();
        d.insert("users", rec("u1")).await.unwrap();
        let r = d.insert("users", rec("u1")).await;
        assert!(matches!(r, Err(DatabaseError::AlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_update_existing_record_succeeds() {
        let d = db();
        d.insert("users", rec("u1")).await.unwrap();
        let updated = rec("u1").set("name", Value::String("Alice".into()));
        let r = d.update("users", "u1", updated).await.unwrap();
        assert_eq!(r.rows_affected, 1);
    }

    #[tokio::test]
    async fn test_update_nonexistent_record_returns_not_found_error() {
        let d = db();
        let r = d.update("users", "missing", rec("missing")).await;
        assert!(matches!(r, Err(DatabaseError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_removes_existing_record() {
        let d = db();
        d.insert("users", rec("u1")).await.unwrap();
        let r = d.delete("users", "u1").await.unwrap();
        assert_eq!(r.rows_affected, 1);
        assert!(!d.exists("users", "u1").await.unwrap());
    }

    #[tokio::test]
    async fn test_count_returns_number_of_inserted_records() {
        let d = db();
        d.insert("items", rec("i1")).await.unwrap();
        d.insert("items", rec("i2")).await.unwrap();
        assert_eq!(d.count("items").await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_query_with_filter_returns_matching_records() {
        let d = db();
        d.insert("orders", rec("o1").set("status", Value::String("active".into()))).await.unwrap();
        d.insert("orders", rec("o2").set("status", Value::String("closed".into()))).await.unwrap();
        let params = QueryParams::table("orders")
            .with_filter("status", Value::String("active".into()));
        let results = d.query(params).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_batch_insert_inserts_all_records() {
        let d = db();
        let records = vec![rec("b1"), rec("b2"), rec("b3")];
        let r = d.batch_insert("batch", records).await.unwrap();
        assert_eq!(r.rows_affected, 3);
        assert_eq!(d.count("batch").await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_health_check_returns_healthy() {
        let r = db().health_check().await.unwrap();
        assert_eq!(r.status, crate::api::port::database_read::DbHealthStatus::Healthy);
    }
}
