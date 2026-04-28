//! API layer — config, error, reserved-key constants.

pub(crate) mod bearer_auth_config;
pub(crate) mod bearer_auth_error;
pub(crate) mod metadata_keys;

pub use bearer_auth_config::{BearerInboundConfig, BearerOutboundConfig, BearerSecret};
pub use bearer_auth_error::BearerAuthError;
pub use metadata_keys::{AUTHORIZATION_HEADER, EXTRACTED_BEARER_SUBJECT};
