//! Local filesystem FileInbound implementation.

use std::pin::Pin;

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::stream::Stream;

use crate::api::port::file_inbound::{FileHealthCheck, FileInbound, FileInboundError, FileInboundResult};
use crate::api::value_object::{FileInfo, FileStorageConfig, ListOptions, ListResult, PresignedUrl};

/// Reads files from the local filesystem.
pub struct LocalFileSource {
    pub(crate) config: FileStorageConfig,
}

impl LocalFileSource {
    /// Create a new `LocalFileSource` with the given storage configuration.
    pub fn new(config: FileStorageConfig) -> Self {
        Self { config }
    }
}

impl FileInbound for LocalFileSource {
    fn read(&self, path: &str) -> BoxFuture<'_, FileInboundResult<Vec<u8>>> {
        let full_path = format!("{}/{}", self.config.base_path.trim_end_matches('/'), path.trim_start_matches('/'));
        Box::pin(async move {
            std::fs::read(&full_path).map_err(FileInboundError::IoError)
        })
    }

    fn metadata(&self, path: &str) -> BoxFuture<'_, FileInboundResult<FileInfo>> {
        let full_path = format!("{}/{}", self.config.base_path.trim_end_matches('/'), path.trim_start_matches('/'));
        let path_owned = path.to_string();
        Box::pin(async move {
            let meta = std::fs::metadata(&full_path).map_err(FileInboundError::IoError)?;
            let size = meta.len();
            let last_modified: DateTime<Utc> = meta.modified()
                .map(DateTime::from)
                .unwrap_or_else(|_| Utc::now());
            let is_directory = meta.is_dir();
            let mut info = FileInfo::new(path_owned, size);
            info.last_modified = last_modified;
            info.is_directory = is_directory;
            Ok(info)
        })
    }

    fn list(&self, options: ListOptions) -> BoxFuture<'_, FileInboundResult<ListResult>> {
        let base = self.config.base_path.clone();
        let prefix = options.prefix.clone().unwrap_or_default();
        let max_results = options.max_results;
        Box::pin(async move {
            let root = format!("{}/{}", base.trim_end_matches('/'), prefix.trim_start_matches('/'));
            let mut files = Vec::new();
            let read_dir = std::fs::read_dir(&root).map_err(FileInboundError::IoError)?;
            for entry in read_dir {
                let entry = entry.map_err(FileInboundError::IoError)?;
                let path = entry.path();
                let rel = path.to_string_lossy().into_owned();
                let meta = entry.metadata().map_err(FileInboundError::IoError)?;
                let size = meta.len();
                files.push(FileInfo::new(rel, size));
                if let Some(max) = max_results {
                    if files.len() >= max { break; }
                }
            }
            Ok(ListResult { files, prefixes: vec![], next_continuation_token: None, is_truncated: false })
        })
    }

    fn exists(&self, path: &str) -> BoxFuture<'_, FileInboundResult<bool>> {
        let full_path = format!("{}/{}", self.config.base_path.trim_end_matches('/'), path.trim_start_matches('/'));
        Box::pin(async move {
            Ok(std::path::Path::new(&full_path).exists())
        })
    }

    fn presigned_read_url(&self, path: &str, _expires_in_secs: u64) -> BoxFuture<'_, FileInboundResult<PresignedUrl>> {
        let full_path = format!("file://{}/{}", self.config.base_path.trim_end_matches('/'), path.trim_start_matches('/'));
        Box::pin(async move {
            Ok(PresignedUrl {
                url: full_path,
                expires_at: Utc::now() + chrono::Duration::seconds(3600),
                method: "GET".into(),
            })
        })
    }

    fn health_check(&self) -> BoxFuture<'_, FileInboundResult<FileHealthCheck>> {
        let base = self.config.base_path.clone();
        Box::pin(async move {
            if std::path::Path::new(&base).exists() {
                Ok(FileHealthCheck::healthy())
            } else {
                Ok(FileHealthCheck::unhealthy(format!("base path not found: {base}")))
            }
        })
    }

    #[allow(clippy::type_complexity)]
    fn list_stream(
        &self,
        options: ListOptions,
    ) -> BoxFuture<'_, FileInboundResult<Pin<Box<dyn Stream<Item = FileInboundResult<FileInfo>> + Send + '_>>>>
    {
        Box::pin(async move {
            let result = self.list(options).await?;
            let stream: Pin<Box<dyn Stream<Item = FileInboundResult<FileInfo>> + Send + '_>> =
                Box::pin(futures::stream::iter(result.files.into_iter().map(Ok)));
            Ok(stream)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::value_object::FileStorageConfig;

    #[tokio::test]
    async fn test_exists_returns_true_for_current_directory() {
        let src = LocalFileSource::new(FileStorageConfig::local("."));
        let result = src.exists("").await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_read_returns_error_for_missing_file() {
        let src = LocalFileSource::new(FileStorageConfig::local("."));
        let result = src.read("__nonexistent_file_xyz__.bin").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check_returns_healthy_for_existing_base_path() {
        let src = LocalFileSource::new(FileStorageConfig::local("."));
        let h = src.health_check().await.unwrap();
        assert!(h.healthy);
    }

    #[tokio::test]
    async fn test_health_check_returns_unhealthy_for_missing_base_path() {
        let src = LocalFileSource::new(FileStorageConfig::local("/__nonexistent_dir_xyz__"));
        let h = src.health_check().await.unwrap();
        assert!(!h.healthy);
    }
}
