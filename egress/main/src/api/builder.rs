//! Builder type for outbound adapter configuration.

use std::sync::Arc;

use swe_edge_egress_database::DatabaseGateway;

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
    Arc::new(swe_edge_egress_database::memory_database_impl())
}

#[cfg(test)]
mod tests {
    use super::*;
    use swe_edge_egress_database::Record;
    use serde_json::Value;

    #[test]
    fn test_builder_new_returns_default() {
        let _ = Builder::new();
    }

    /// @covers: build_memory_database
    #[tokio::test]
    async fn test_build_memory_database_returns_arc_database_gateway() {
        let db = build_memory_database();
        let rec = Record::new().set("id", Value::String("x".into()));
        db.insert("t", rec).await.unwrap();
        let found = db.get_by_id("t", "x").await.unwrap();
        assert!(found.is_some());
    }
}
