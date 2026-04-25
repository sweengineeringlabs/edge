//! SAF builder — constructs outbound adapters.

use std::sync::Arc;

use crate::api::database::DatabaseGateway;
use crate::core::database::MemoryDatabase;

/// Returns an in-memory database adapter (for testing/development).
pub fn memory_database() -> impl DatabaseGateway {
    MemoryDatabase::new()
}

/// Builder for outbound adapter configuration.
#[derive(Debug, Default)]
pub struct Builder;

impl Builder {
    /// Construct with default configuration.
    pub fn new() -> Self {
        Self
    }

    /// Build the in-memory database adapter.
    pub fn build_memory_database(self) -> Arc<dyn DatabaseGateway> {
        Arc::new(MemoryDatabase::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::database::{DatabaseRead, DatabaseWrite};

    /// @covers: memory_database
    #[test]
    fn test_memory_database_returns_database_gateway() {
        let db = memory_database();
        db.put("k", b"v").unwrap();
        assert_eq!(db.get("k").unwrap(), Some(b"v".to_vec()));
    }

    /// @covers: Builder::new
    #[test]
    fn test_new_constructs_builder() {
        let _ = Builder::new();
    }

    /// @covers: Builder::build_memory_database
    #[test]
    fn test_build_memory_database_returns_arc_database_gateway() {
        let db = Builder::new().build_memory_database();
        db.put("x", b"1").unwrap();
        assert_eq!(db.get("x").unwrap(), Some(b"1".to_vec()));
    }
}
