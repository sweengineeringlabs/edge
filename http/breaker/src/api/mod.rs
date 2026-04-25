//! API layer — public schema + trait contracts + public types.
pub mod builder;
pub(crate) mod breaker_config;
pub(crate) mod breaker_layer;
pub(crate) mod breaker_state;
pub(crate) mod default_http_breaker;
pub(crate) mod error;
pub(crate) mod host_breaker;
pub(crate) mod http_breaker;
pub(crate) mod traits;

