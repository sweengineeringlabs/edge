//! Egress configuration types.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error loading egress configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("parse error: {0}")]
    ParseError(String),
}

/// Output format for a sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SinkFormat {
    Json,
    Csv,
    Ndjson,
    Raw,
}

/// Type of output sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SinkType {
    Stdout,
    File,
    Http,
    Database,
}

/// Configuration for an output sink.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkConfig {
    pub sink_type: SinkType,
    pub format: SinkFormat,
    pub target: Option<String>,
}

impl SinkConfig {
    pub fn stdout() -> Self {
        Self { sink_type: SinkType::Stdout, format: SinkFormat::Json, target: None }
    }

    pub fn file(path: impl Into<String>) -> Self {
        Self { sink_type: SinkType::File, format: SinkFormat::Json, target: Some(path.into()) }
    }
}

/// Top-level egress configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EgressConfig {
    pub sinks: Vec<SinkConfig>,
}

impl EgressConfig {
    pub fn with_sink(mut self, sink: SinkConfig) -> Self {
        self.sinks.push(sink); self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: stdout
    #[test]
    fn test_stdout_creates_stdout_sink_config() {
        let s = SinkConfig::stdout();
        assert_eq!(s.sink_type, SinkType::Stdout);
        assert!(s.target.is_none());
    }

    /// @covers: file
    #[test]
    fn test_file_creates_file_sink_config_with_path() {
        let s = SinkConfig::file("/tmp/out.json");
        assert_eq!(s.sink_type, SinkType::File);
        assert_eq!(s.target, Some("/tmp/out.json".to_string()));
    }

    /// @covers: with_sink
    #[test]
    fn test_with_sink_appends_to_egress_config() {
        let cfg = EgressConfig::default().with_sink(SinkConfig::stdout());
        assert_eq!(cfg.sinks.len(), 1);
    }
}
