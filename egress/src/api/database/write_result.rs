//! Result of a database write operation.

use serde::{Deserialize, Serialize};

/// Result of a database write (insert/update/delete) operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub rows_affected: u64,
    pub last_insert_id: Option<String>,
}

impl WriteResult {
    pub fn rows(rows_affected: u64) -> Self {
        Self { rows_affected, last_insert_id: None }
    }

    pub fn inserted(id: impl Into<String>) -> Self {
        Self { rows_affected: 1, last_insert_id: Some(id.into()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: rows
    #[test]
    fn test_rows_creates_write_result_with_row_count() {
        let r = WriteResult::rows(3);
        assert_eq!(r.rows_affected, 3);
        assert!(r.last_insert_id.is_none());
    }

    /// @covers: inserted
    #[test]
    fn test_inserted_creates_write_result_with_id() {
        let r = WriteResult::inserted("uuid-123");
        assert_eq!(r.rows_affected, 1);
        assert_eq!(r.last_insert_id, Some("uuid-123".to_string()));
    }
}
