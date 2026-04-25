//! Database egress api — read, write, and gateway traits.
pub(crate) mod database_gateway;
pub(crate) mod database_read;
pub(crate) mod database_write;

pub use database_gateway::DatabaseGateway;
pub use database_read::DatabaseRead;
pub use database_write::DatabaseWrite;
