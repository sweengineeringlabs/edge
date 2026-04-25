//! Query parameters for database read operations.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parameters for a database query.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryParams {
    pub table: String,
    pub filters: Vec<(String, Value)>,
    pub order_by: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl QueryParams {
    pub fn table(table: impl Into<String>) -> Self {
        Self { table: table.into(), ..Default::default() }
    }

    pub fn with_filter(mut self, field: impl Into<String>, value: Value) -> Self {
        self.filters.push((field.into(), value)); self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit); self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset); self
    }

    pub fn order_by(mut self, field: impl Into<String>) -> Self {
        self.order_by = Some(field.into()); self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: table
    #[test]
    fn test_table_creates_query_for_table() {
        let q = QueryParams::table("users");
        assert_eq!(q.table, "users");
        assert!(q.filters.is_empty());
    }

    /// @covers: with_filter
    #[test]
    fn test_with_filter_appends_filter_condition() {
        let q = QueryParams::table("orders").with_filter("status", Value::String("active".into()));
        assert_eq!(q.filters.len(), 1);
        assert_eq!(q.filters[0].0, "status");
    }

    /// @covers: with_limit
    #[test]
    fn test_with_limit_sets_limit() {
        let q = QueryParams::table("t").with_limit(25);
        assert_eq!(q.limit, Some(25));
    }
}
