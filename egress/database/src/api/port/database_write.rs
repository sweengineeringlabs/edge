//! DatabaseWrite trait — async writes to a database.

use futures::future::BoxFuture;

use crate::api::aggregate::Record;
use crate::api::value_object::{QueryParams, WriteResult};
use super::database_read::DatabaseResult;

/// Writes records to a database.
pub trait DatabaseWrite: Send + Sync {
    fn insert(&self, table: &str, record: Record) -> BoxFuture<'_, DatabaseResult<WriteResult>>;
    fn update(&self, table: &str, id: &str, record: Record) -> BoxFuture<'_, DatabaseResult<WriteResult>>;
    fn delete(&self, table: &str, id: &str) -> BoxFuture<'_, DatabaseResult<WriteResult>>;
    fn batch_insert(&self, table: &str, records: Vec<Record>) -> BoxFuture<'_, DatabaseResult<WriteResult>>;
    fn update_where(&self, table: &str, params: QueryParams, record: Record) -> BoxFuture<'_, DatabaseResult<WriteResult>>;
    fn delete_where(&self, table: &str, params: QueryParams) -> BoxFuture<'_, DatabaseResult<WriteResult>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_write_is_object_safe() {
        fn _assert_object_safe(_: &dyn DatabaseWrite) {}
    }
}
