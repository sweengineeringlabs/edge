//! File output sink.

use std::path::PathBuf;

use futures::future::BoxFuture;

use crate::api::file::UploadOptions;
use crate::api::output::OutputSink;
use crate::api::traits::FileOutbound;
use crate::api::types::GatewayResult;
use crate::saf::config::GatewayConfig;

/// Writes output data to a file via the file gateway.
pub(crate) struct FileSink {
    path: PathBuf,
}

impl FileSink {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl OutputSink for FileSink {
    fn write(&self, data: &[u8]) -> BoxFuture<'_, GatewayResult<()>> {
        let parent = self
            .path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        let filename = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("output")
            .to_string();
        let data = data.to_vec();

        Box::pin(async move {
            let config = GatewayConfig::default()
                .with_file(|f| f.base_path = parent.to_string_lossy().into_owned());
            let gw = config.file_gateway();
            gw.write(&filename, data, UploadOptions::overwrite()).await?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_sink_creates_file() {
        let dir = std::env::temp_dir().join("swe_gw_output_sink_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("output.txt");

        let sink = FileSink::new(path.clone());
        sink.write(b"file sink test").await.unwrap();

        assert!(path.exists());
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "file sink test");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_file_sink_creates_parent_dirs() {
        let dir = std::env::temp_dir().join("swe_gw_output_sink_parents");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("a").join("b").join("out.txt");

        let sink = FileSink::new(path.clone());
        sink.write(b"nested").await.unwrap();

        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
