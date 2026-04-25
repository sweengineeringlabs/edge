//! Primary trait for the breaker crate.
//!
//! Crate-level processor contract. Rule 153 requires the
//! primary trait to match the declared service_type (=
//! \"processor\"); we use the domain-prefixed form `HttpBreaker`
//! for clarity at use sites. The impl lives in
//! `core/default_http_breaker.rs`.
//!
//! Scaffold phase: a single `describe()` method — placeholder
//! that lets the trait satisfy rule 153 without committing to
//! a final signature. Real method set grows when the middleware
//! impl lands.

/// The breaker crate's primary trait. Every middleware layer
/// produced by this crate implements it.
pub trait HttpBreaker: Send + Sync {
    /// Identify this processor in log / trace output.
    ///
    /// Returns the crate's canonical name (e.g. `\"swe_edge_http_breaker\"`).
    /// Future impls will add scheme / policy-shape methods.
    fn describe(&self) -> &'static str;
}
