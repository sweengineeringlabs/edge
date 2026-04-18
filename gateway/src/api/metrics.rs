//! Metrics contract — field envelope, collector trait, and extractor alias.
//!
//! The runtime middleware that consumes these types lives in
//! `crate::core::metrics_bridge` as `pub(crate)` (rule 50). Consumers
//! construct one via `saf::metrics_middleware(collector, extractor)`,
//! which returns `impl ResponseMiddleware` so the concrete bridge type
//! is never named externally (rules 47, 159).
//!
//! Rules enforced by this placement:
//!   - rule 50: the bridge struct stays `pub(crate)` in core/.
//!   - rule 159: saf/ signatures take / return these api/ types.
//!   - rule 160: consumers never name a core type — only api/ types.

use std::sync::Arc;

/// Extracted metric fields that the bridge knows how to record.
///
/// The extractor closure is responsible for pulling these values out of the
/// domain-specific response payload.
#[derive(Debug, Clone)]
pub struct MetricFields {
    /// Identifier for the upstream provider (e.g. "openai", "anthropic").
    pub provider: String,
    /// Model identifier (e.g. "gpt-4", "claude-3").
    pub model: String,
    /// Status label (e.g. "ok", "error").
    pub status: String,
    /// Latency in seconds.
    pub latency_secs: f64,
    /// Number of input/prompt tokens consumed.
    pub input_tokens: u64,
    /// Number of output/completion tokens produced.
    pub output_tokens: u64,
}

/// Trait for a generic metrics collector that the bridge delegates to.
///
/// Implementors connect this to their observability stack (Prometheus,
/// OpenTelemetry, in-memory counters for testing, etc.).
pub trait MetricsCollector: Send + Sync {
    /// Record a single completion event with the given fields.
    fn record_completion(
        &self,
        provider: &str,
        model: &str,
        status: &str,
        latency_secs: f64,
        input_tokens: u64,
        output_tokens: u64,
    );
}

/// Type alias for the field-extractor closure.
///
/// Given a response `serde_json::Value`, returns `Some(MetricFields)` if
/// the response contains enough data to record metrics, or `None` to skip.
pub type FieldExtractor =
    Arc<dyn Fn(&serde_json::Value) -> Option<MetricFields> + Send + Sync>;
