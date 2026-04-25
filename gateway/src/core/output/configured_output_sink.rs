//! Config-driven output sink.

use futures::future::BoxFuture;

use crate::api::output::OutputSink;
use crate::api::types::{GatewayError, GatewayResult};
use crate::saf::config::{GatewayConfig, SinkType};

use super::file_sink::FileSink;
use super::stdout_sink::StdoutSink;

/// Config-driven sink that dispatches to stdout or file based on `gateway.toml`.
pub(crate) struct ConfiguredOutputSink {
    config: GatewayConfig,
}

impl ConfiguredOutputSink {
    pub(crate) fn new(config: GatewayConfig) -> Self {
        Self { config }
    }
}

impl OutputSink for ConfiguredOutputSink {
    fn write(&self, data: &[u8]) -> BoxFuture<'_, GatewayResult<()>> {
        let sink_type = self.config.sink.sink_type;
        let path = self.config.sink.path.clone();
        let data = data.to_vec();

        Box::pin(async move {
            match sink_type {
                SinkType::Stdout => StdoutSink.write(&data).await,
                SinkType::File => {
                    let path = path.ok_or_else(|| {
                        GatewayError::invalid_input(
                            "sink_type is 'file' but no 'path' specified in gateway.toml",
                        )
                    })?;
                    FileSink::new(path).write(&data).await
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_configured_sink_stdout() {
        let config = GatewayConfig::default();
        let sink = ConfiguredOutputSink::new(config);
        let result = sink.write(b"configured stdout\n").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_configured_sink_file() {
        let dir = std::env::temp_dir().join("swe_gw_configured_sink_test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("report.json");

        let config = GatewayConfig::default().with_sink(|s| {
            s.sink_type = SinkType::File;
            s.path = Some(path.clone());
        });
        let sink = ConfiguredOutputSink::new(config);
        sink.write(b"{\"test\": true}").await.unwrap();

        assert!(path.exists());
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "{\"test\": true}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_configured_sink_file_missing_path() {
        let config = GatewayConfig::default().with_sink(|s| {
            s.sink_type = SinkType::File;
            s.path = None;
        });
        let sink = ConfiguredOutputSink::new(config);
        let result = sink.write(b"data").await;
        assert!(result.is_err());
    }
}
