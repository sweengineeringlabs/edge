//! API layer — inbound trait contracts and public types.
pub mod builder;
pub(crate) mod ingress_error;
pub(crate) mod file;
pub(crate) mod file_inbound;
pub(crate) mod http_inbound;
pub(crate) mod inbound_source;
pub(crate) mod traits;
pub(crate) mod validator;
