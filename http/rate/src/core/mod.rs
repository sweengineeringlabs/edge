//! Core layer — token-bucket impl + middleware + default impl
//! of the primary api trait.

pub(crate) mod default_http_rate;
pub(crate) mod rate_layer;
pub(crate) mod token_bucket;
