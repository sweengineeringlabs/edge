//! Trait definitions for the swe-http-main crate.

/// Marker trait for the middleware stack assembler.
pub(crate) trait StackAssembler: Send + Sync {}
