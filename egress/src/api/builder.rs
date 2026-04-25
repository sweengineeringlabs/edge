//! Builder type for outbound adapter configuration.

use std::sync::Arc;

use crate::api::database::DatabaseGateway;

/// Builder for outbound adapter configuration.
#[derive(Debug, Default)]
pub struct Builder;

impl Builder {
    /// Construct with default configuration.
    pub fn new() -> Self {
        Self
    }
}

/// Build the in-memory database adapter.
pub fn build_memory_database() -> Arc<dyn DatabaseGateway> {
    Arc::new(crate::core::database::MemoryDatabase::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_new_returns_default() {
        let _ = Builder::new();
    }

    /// @covers: build_memory_database
    #[test]
    fn test_build_memory_database_returns_arc_database_gateway() {
        let db = build_memory_database();
        db.put("x", b"1").unwrap();
        assert_eq!(db.get("x").unwrap(), Some(b"1".to_vec()));
    }
}
