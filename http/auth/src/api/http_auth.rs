//! Primary trait for the auth crate (rule 153).
//!
//! Scaffold phase: a single `describe()` method. Phase 2 (real
//! middleware impl) adds `process(&self, req: &mut Request)`
//! as a trait extension.

/// Auth processor contract. Every middleware layer this crate
/// produces implements it.
pub(crate) trait HttpAuth: Send + Sync {
    /// Identify this processor in log / trace output.
    fn describe(&self) -> &'static str;
}
