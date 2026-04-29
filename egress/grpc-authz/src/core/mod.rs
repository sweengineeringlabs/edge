//! Core layer — interceptor + the built-in method-ACL policy.

pub(crate) mod authz_interceptor;
pub(crate) mod method_acl_policy;

pub use authz_interceptor::AuthzInterceptor;
pub use method_acl_policy::MethodAclPolicy;
