//! SAF builder — constructs outbound adapters.

use crate::api::database::DatabaseGateway;
use crate::api::traits::Validator;
use crate::core::database::MemoryDatabase;
use crate::core::validator::PassthroughValidator;

/// Returns an in-memory database adapter (for testing/development).
pub fn memory_database() -> impl DatabaseGateway {
    MemoryDatabase::new()
}

/// Returns the default passthrough validator (accepts all output).
pub fn passthrough_validator() -> impl Validator {
    PassthroughValidator
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::database::DatabaseRead;
    use crate::api::database::DatabaseWrite;

    /// @covers: memory_database
    #[test]
    fn test_memory_database_returns_database_gateway() {
        let db = memory_database();
        db.put("k", b"v").unwrap();
        assert_eq!(db.get("k").unwrap(), Some(b"v".to_vec()));
    }

    /// @covers: passthrough_validator
    #[test]
    fn test_passthrough_validator_accepts_all_output() {
        let v = passthrough_validator();
        assert!(v.is_valid("payload"));
        assert!(v.is_valid(""));
    }
}
