//! DatabaseRead trait — async reads from a database.

use futures::future::BoxFuture;

use crate::api::egress_error::EgressResult;
use crate::api::health_check::HealthCheck;

use super::query_params::QueryParams;
use super::record::Record;

/// Reads records from a database.
pub trait DatabaseRead: Send + Sync {
    fn query(&self, params: QueryParams) -> BoxFuture<'_, EgressResult<Vec<Record>>>;
    fn get_by_id(&self, table: &str, id: &str) -> BoxFuture<'_, EgressResult<Option<Record>>>;
    fn exists(&self, table: &str, id: &str) -> BoxFuture<'_, EgressResult<bool>>;
    fn count(&self, table: &str) -> BoxFuture<'_, EgressResult<u64>>;
    fn health_check(&self) -> BoxFuture<'_, EgressResult<HealthCheck>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_read_is_object_safe() {
        fn _assert_object_safe(_: &dyn DatabaseRead) {}
    }
}
