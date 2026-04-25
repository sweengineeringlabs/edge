//! Core layer — per-host state machine + middleware impl +
//! default impl of the primary api trait.

pub(crate) mod breaker_layer;
pub(crate) mod default_http_breaker;
pub(crate) mod host_breaker;
