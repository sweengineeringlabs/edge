//! DatabaseWrite trait — writes records to a database.

use crate::api::egress_error::EgressError;

/// Writes records to a database.
pub trait DatabaseWrite: Send + Sync {
    /// A description of this database adapter for diagnostics.
    fn describe(&self) -> &'static str;

    /// Insert or update a record.
    fn put(&self, key: &str, value: &[u8]) -> Result<(), EgressError>;

    /// Delete a record by key. No-op if the key does not exist.
    fn delete(&self, key: &str) -> Result<(), EgressError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NullWriter;
    impl DatabaseWrite for NullWriter {
        fn describe(&self) -> &'static str { "null" }
        fn put(&self, _key: &str, _value: &[u8]) -> Result<(), EgressError> { Ok(()) }
        fn delete(&self, _key: &str) -> Result<(), EgressError> { Ok(()) }
    }

    #[test]
    fn test_database_write_put_succeeds() {
        assert!(NullWriter.put("k", b"v").is_ok());
    }

    #[test]
    fn test_database_write_delete_succeeds() {
        assert!(NullWriter.delete("k").is_ok());
    }
}
