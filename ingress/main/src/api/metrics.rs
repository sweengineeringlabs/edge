//! Metrics contract — collector trait and field envelope.

use std::sync::Arc;

/// Extracted metric fields from a model completion call.
#[derive(Debug, Clone)]
pub struct MetricFields {
    /// Provider name (e.g. `"openai"`).
    pub provider: String,
    /// Model identifier (e.g. `"gpt-4"`).
    pub model: String,
    /// Completion status (e.g. `"ok"`, `"error"`).
    pub status: String,
    /// Wall-clock latency of the call in seconds.
    pub latency_secs: f64,
    /// Number of input tokens consumed.
    pub input_tokens: u64,
    /// Number of output tokens produced.
    pub output_tokens: u64,
}

/// Trait for a generic metrics collector.
pub trait MetricsCollector: Send + Sync {
    /// Record a single model completion event.
    fn record_completion(&self, provider: &str, model: &str, status: &str, latency_secs: f64, input_tokens: u64, output_tokens: u64);
}

/// Type alias for the field-extractor closure.
pub type FieldExtractor = Arc<dyn Fn(&serde_json::Value) -> Option<MetricFields> + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_is_object_safe() {
        fn _accepts(_m: &dyn MetricsCollector) {}
    }

    #[test]
    fn test_metric_fields_holds_all_fields() {
        let f = MetricFields { provider: "openai".into(), model: "gpt-4".into(), status: "ok".into(), latency_secs: 0.5, input_tokens: 100, output_tokens: 200 };
        assert_eq!(f.provider, "openai");
        assert_eq!(f.input_tokens, 100);
    }
}
