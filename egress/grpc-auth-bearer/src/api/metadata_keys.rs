//! Reserved metadata keys used by the bearer interceptors.

/// Standard HTTP/2 / gRPC `authorization` metadata key (lower-case).
pub const AUTHORIZATION_HEADER: &str = "authorization";

/// Internal metadata key under which a successfully validated JWT
/// `sub` claim is republished by [`crate::BearerInboundInterceptor`]
/// for downstream authz policies.  Treated as **trusted** only when
/// set by this interceptor — the inbound interceptor ALWAYS strips
/// any incoming value before it (re-)inserts the verified one.
pub const EXTRACTED_BEARER_SUBJECT: &str = "x-edge-extracted-bearer-subject";
