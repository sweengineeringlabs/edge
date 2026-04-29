//! API layer — config + error types for the mTLS interceptor.

pub(crate) mod mtls_auth_config;
pub(crate) mod mtls_auth_error;

pub use mtls_auth_config::MtlsAuthConfig;
pub use mtls_auth_error::MtlsAuthError;
