//! API layer — public schema + trait declarations + public types.
//!
//! Per SEA rule 160, public type *declarations* live here.  Impl
//! blocks live in `core/`.  Concrete structs that consumers
//! construct (e.g. [`GrpcRetryConfig`], [`GrpcRetryClient`]) are
//! declared here so the public type identity is owned by api/.

pub(crate) mod backoff;
pub(crate) mod builder;
pub(crate) mod error;
pub(crate) mod retry_client;
pub(crate) mod retry_config;
pub(crate) mod retry_policy;
pub(crate) mod traits;
