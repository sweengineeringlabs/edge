//! DatabaseGateway trait — composes read and write access to a database.

use super::database_read::DatabaseRead;
use super::database_write::DatabaseWrite;

/// Composes read and write access to a single database backend.
pub trait DatabaseGateway: DatabaseRead + DatabaseWrite {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::egress_error::EgressError;

    struct NullDb;
    impl DatabaseRead for NullDb {
        fn describe(&self) -> &'static str { "null" }
        fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, EgressError> { Ok(None) }
        fn list(&self, _prefix: Option<&str>) -> Result<Vec<String>, EgressError> { Ok(vec![]) }
    }
    impl DatabaseWrite for NullDb {
        fn describe(&self) -> &'static str { "null" }
        fn put(&self, _key: &str, _value: &[u8]) -> Result<(), EgressError> { Ok(()) }
        fn delete(&self, _key: &str) -> Result<(), EgressError> { Ok(()) }
    }
    impl DatabaseGateway for NullDb {}

    #[test]
    fn test_database_gateway_put_and_get_round_trips_on_null() {
        let db = NullDb;
        assert!(db.put("k", b"v").is_ok());
        assert_eq!(db.get("k").unwrap(), None);
    }
}
