//! API layer — outbound trait contracts and public types.
pub mod builder;
pub(crate) mod database;
pub(crate) mod egress_error;
pub(crate) mod file_outbound;
pub(crate) mod http_client;
pub(crate) mod notification_sender;
pub(crate) mod outbound_sink;
pub(crate) mod payment_gateway;
pub(crate) mod traits;
pub(crate) mod validator;
