//! DatabaseGateway trait — composes read and write access to a database.

use super::database_read::DatabaseRead;
use super::database_write::DatabaseWrite;

/// Composes read and write access to a single database backend.
pub trait DatabaseGateway: DatabaseRead + DatabaseWrite {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_gateway_is_object_safe() {
        fn _assert_object_safe(_: &dyn DatabaseGateway) {}
    }
}
