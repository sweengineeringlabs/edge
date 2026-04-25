//! Database egress traits — read and write adapters.

use crate::api::error::EgressError;

/// Reads records from a database.
pub trait DatabaseRead: Send + Sync {
    /// A description of this database adapter for diagnostics.
    fn describe(&self) -> &'static str;

    /// Fetch a single record by its string key. Returns `None` if absent.
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, EgressError>;

    /// List all keys matching an optional prefix filter.
    fn list(&self, prefix: Option<&str>) -> Result<Vec<String>, EgressError>;
}

/// Writes records to a database.
pub trait DatabaseWrite: Send + Sync {
    /// A description of this database adapter for diagnostics.
    fn describe(&self) -> &'static str;

    /// Insert or update a record.
    fn put(&self, key: &str, value: &[u8]) -> Result<(), EgressError>;

    /// Delete a record by key. No-op if the key does not exist.
    fn delete(&self, key: &str) -> Result<(), EgressError>;
}

/// Composes read and write access to a single database backend.
pub trait DatabaseGateway: DatabaseRead + DatabaseWrite {}
