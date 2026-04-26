//! Core layer — credential resolution + strategy impls +
//! the middleware that composes them.

pub(crate) mod auth_middleware;
pub(crate) mod credential;
pub(crate) mod default_http_auth;
pub(crate) mod strategy;
