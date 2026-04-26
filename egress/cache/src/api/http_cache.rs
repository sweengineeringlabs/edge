//! Primary trait for the cache crate.
//!
//! Crate-level processor contract. Rule 153 requires the
//! primary trait to match the declared service_type (=
//! \"processor\"); we use the domain-prefixed form `HttpCache`
//! for clarity at use sites. The impl lives in
//! `core/default_http_cache.rs`.
//!
//! Scaffold phase: a single `describe()` method — placeholder
//! that lets the trait satisfy rule 153 without committing to
//! a final signature. Real method set grows when the middleware
//! impl lands.

/// The cache crate's primary trait. Every middleware layer
/// produced by this crate implements it.
pub trait HttpCache: Send + Sync {
    /// Identify this processor in log / trace output.
    ///
    /// Returns the crate's canonical name (e.g. `\"swe_edge_egress_cache\"`).
    /// Future impls will add scheme / policy-shape methods.
    fn describe(&self) -> &'static str;
}
