//! SAF layer — database public facade.

pub use crate::api::aggregate::Record;
pub use crate::api::port::{DatabaseGateway, DatabaseRead, DatabaseWrite};
pub use crate::api::port::database_read::{DatabaseError, DatabaseResult, DbHealthCheck, DbHealthStatus};
pub use crate::api::value_object::{DatabaseConfig, DatabaseType, IsolationLevel, QueryParams, WriteResult};

/// Returns an in-memory database adapter (for testing/development).
///
/// Not named `memory_database` to avoid conflict with the main crate re-export.
pub fn memory_database_impl() -> impl DatabaseGateway {
    crate::core::database::MemoryDatabase::new()
}
