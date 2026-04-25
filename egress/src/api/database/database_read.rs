//! DatabaseRead trait — reads records from a database.

use crate::api::egress_error::EgressError;

/// Reads records from a database.
pub trait DatabaseRead: Send + Sync {
    /// A description of this database adapter for diagnostics.
    fn describe(&self) -> &'static str;

    /// Fetch a single record by its string key. Returns `None` if absent.
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, EgressError>;

    /// List all keys matching an optional prefix filter.
    fn list(&self, prefix: Option<&str>) -> Result<Vec<String>, EgressError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NullReader;
    impl DatabaseRead for NullReader {
        fn describe(&self) -> &'static str { "null" }
        fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, EgressError> { Ok(None) }
        fn list(&self, _prefix: Option<&str>) -> Result<Vec<String>, EgressError> { Ok(vec![]) }
    }

    #[test]
    fn test_database_read_get_missing_key_returns_none() {
        assert_eq!(NullReader.get("x").unwrap(), None);
    }

    #[test]
    fn test_database_read_list_returns_empty() {
        assert!(NullReader.list(None).unwrap().is_empty());
    }
}
