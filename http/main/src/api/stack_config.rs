//! Aggregate config schema for the full middleware stack.

/// Aggregates per-crate configs for one-shot stack assembly.
#[derive(Debug, Clone)]
pub struct StackConfig {
    /// Auth middleware policy.
    pub auth: swe_edge_http_auth::AuthConfig,
}
