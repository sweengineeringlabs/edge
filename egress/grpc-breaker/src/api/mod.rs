//! API layer — public schema + trait declarations + public types.
//!
//! Per SEA rule 160, public type *declarations* live here.  Impl
//! blocks live in `core/`.

pub(crate) mod breaker_client;
pub(crate) mod breaker_config;
pub(crate) mod breaker_state;
pub(crate) mod builder;
pub(crate) mod error;
pub(crate) mod failure_kind;
pub(crate) mod traits;
pub(crate) mod transitions;
