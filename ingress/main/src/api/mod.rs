//! API layer — inbound trait contracts and public types.
pub mod builder;
pub(crate) mod ingress_error;
pub(crate) mod daemon;
pub(crate) mod health_check;
pub(crate) mod inbound_source;
pub(crate) mod metrics;
pub(crate) mod pagination;
pub(crate) mod pipeline;
pub(crate) mod rate_limiter;
pub(crate) mod traits;
pub(crate) mod validator;
