//! API layer — public schema + trait contracts.
//!
//! File layout follows rs.rules 161 (one pub item per file,
//! stem matches snake_case type name). `traits.rs` is kept as
//! a re-export hub per rule 161's resolution note for rule 48.
pub(crate) mod cache_config;
pub(crate) mod error;
pub(crate) mod http_cache;
pub(crate) mod traits;
