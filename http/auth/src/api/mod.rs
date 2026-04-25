//! API layer — public schema + trait contracts + public types.
pub(crate) mod auth_config;
pub(crate) mod auth_middleware;
pub(crate) mod auth_strategy;
pub mod builder;
pub(crate) mod credential;
pub(crate) mod credential_resolver;
pub(crate) mod credential_source;
pub(crate) mod default_http_auth;
pub(crate) mod error;
pub(crate) mod http_auth;
pub(crate) mod strategy;
pub(crate) mod traits;
