//! DatabaseWrite trait — async writes to a database.

use futures::future::BoxFuture;

use crate::api::egress_error::EgressResult;

use super::query_params::QueryParams;
use super::record::Record;
use super::write_result::WriteResult;

/// Writes records to a database.
pub trait DatabaseWrite: Send + Sync {
    fn insert(&self, table: &str, record: Record) -> BoxFuture<'_, EgressResult<WriteResult>>;
    fn update(&self, table: &str, id: &str, record: Record) -> BoxFuture<'_, EgressResult<WriteResult>>;
    fn delete(&self, table: &str, id: &str) -> BoxFuture<'_, EgressResult<WriteResult>>;
    fn batch_insert(&self, table: &str, records: Vec<Record>) -> BoxFuture<'_, EgressResult<WriteResult>>;
    fn update_where(&self, table: &str, params: QueryParams, record: Record) -> BoxFuture<'_, EgressResult<WriteResult>>;
    fn delete_where(&self, table: &str, params: QueryParams) -> BoxFuture<'_, EgressResult<WriteResult>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_write_is_object_safe() {
        fn _assert_object_safe(_: &dyn DatabaseWrite) {}
    }
}
