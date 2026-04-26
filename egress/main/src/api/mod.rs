//! API layer — outbound trait contracts and public types.
pub mod builder;
pub(crate) mod config;
pub(crate) mod egress_error;
pub(crate) mod health_check;
pub(crate) mod outbound_sink;
pub(crate) mod pagination;
pub(crate) mod retry;
pub(crate) mod traits;
pub(crate) mod validator;
