//! Primary trait for the rate crate.
//!
//! Crate-level processor contract. Rule 153 requires the
//! primary trait to match the declared service_type (=
//! \"processor\"); we use the domain-prefixed form `HttpRate`
//! for clarity at use sites. The impl lives in
//! `core/default_http_rate.rs`.
//!
//! Scaffold phase: a single `describe()` method — placeholder
//! that lets the trait satisfy rule 153 without committing to
//! a final signature. Real method set grows when the middleware
//! impl lands.

/// The rate crate's primary trait. Every middleware layer
/// produced by this crate implements it.
pub trait HttpRate: Send + Sync {
    /// Identify this processor in log / trace output.
    ///
    /// Returns the crate's canonical name (e.g. `\"swe_http_rate\"`).
    /// Future impls will add scheme / policy-shape methods.
    fn describe(&self) -> &'static str;
}
