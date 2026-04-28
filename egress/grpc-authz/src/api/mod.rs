//! API layer — policy trait, error type, identity adapter.

pub(crate) mod authz_error;
pub(crate) mod authz_policy;
pub(crate) mod method_acl_config;

pub use authz_error::AuthzError;
pub use authz_policy::AuthzPolicy;
pub use method_acl_config::MethodAclConfig;
