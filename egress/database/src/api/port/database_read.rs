//! DatabaseRead trait — async reads from a database.

use futures::future::BoxFuture;
use thiserror::Error;

use crate::api::aggregate::Record;
use crate::api::value_object::{QueryParams};

/// Error type for database operations.
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("internal: {0}")]
    Internal(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
}

/// Result type for database operations.
pub type DatabaseResult<T> = Result<T, DatabaseError>;

/// Health check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbHealthStatus { Healthy, Unhealthy }

/// Simple health check for the database.
#[derive(Debug, Clone)]
pub struct DbHealthCheck {
    pub status: DbHealthStatus,
}

impl DbHealthCheck {
    pub fn healthy() -> Self { Self { status: DbHealthStatus::Healthy } }
}

/// Reads records from a database.
pub trait DatabaseRead: Send + Sync {
    fn query(&self, params: QueryParams) -> BoxFuture<'_, DatabaseResult<Vec<Record>>>;
    fn get_by_id(&self, table: &str, id: &str) -> BoxFuture<'_, DatabaseResult<Option<Record>>>;
    fn exists(&self, table: &str, id: &str) -> BoxFuture<'_, DatabaseResult<bool>>;
    fn count(&self, table: &str) -> BoxFuture<'_, DatabaseResult<u64>>;
    fn health_check(&self) -> BoxFuture<'_, DatabaseResult<DbHealthCheck>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_read_is_object_safe() {
        fn _assert_object_safe(_: &dyn DatabaseRead) {}
    }
}
