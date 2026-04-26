//! Core implementation layer.
//!
//! Holds concrete, domain-agnostic implementations of the `api/` traits.
//! Downstream Controller crates add domain-specific impls alongside (or
//! replace) these defaults.

pub mod null_lifecycle_monitor;
