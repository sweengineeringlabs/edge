//! File inbound trait — reads from file storage.

use std::pin::Pin;

use futures::future::BoxFuture;
use futures::stream::Stream;

use crate::api::value_object::{FileInfo, ListOptions, ListResult, PresignedUrl};

/// Result type for file inbound operations.
pub type FileInboundResult<T> = Result<T, FileInboundError>;

/// Error type for file inbound operations.
#[derive(Debug, thiserror::Error)]
pub enum FileInboundError {
    #[error("internal: {0}")]
    Internal(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Minimal health-check result for the file domain.
#[derive(Debug, Clone)]
pub struct FileHealthCheck {
    pub healthy: bool,
    pub message: Option<String>,
}

impl FileHealthCheck {
    pub fn healthy() -> Self { Self { healthy: true, message: None } }
    pub fn unhealthy(msg: impl Into<String>) -> Self { Self { healthy: false, message: Some(msg.into()) } }
}

/// Inbound operations for file storage (read operations).
pub trait FileInbound: Send + Sync {
    fn read(&self, path: &str) -> BoxFuture<'_, FileInboundResult<Vec<u8>>>;
    fn metadata(&self, path: &str) -> BoxFuture<'_, FileInboundResult<FileInfo>>;
    fn list(&self, options: ListOptions) -> BoxFuture<'_, FileInboundResult<ListResult>>;
    fn exists(&self, path: &str) -> BoxFuture<'_, FileInboundResult<bool>>;
    fn presigned_read_url(&self, path: &str, expires_in_secs: u64) -> BoxFuture<'_, FileInboundResult<PresignedUrl>>;
    fn health_check(&self) -> BoxFuture<'_, FileInboundResult<FileHealthCheck>>;

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

    #[test]
    fn test_file_inbound_is_object_safe() {
        fn _assert_object_safe(_: &dyn FileInbound) {}
    }

    #[test]
    fn test_file_inbound_error_not_found_formats_correctly() {
        let err = FileInboundError::NotFound("missing".into());
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn test_file_health_check_healthy_is_true() {
        let h = FileHealthCheck::healthy();
        assert!(h.healthy);
    }

    #[test]
    fn test_file_health_check_unhealthy_sets_message() {
        let h = FileHealthCheck::unhealthy("disk error");
        assert!(!h.healthy);
        assert_eq!(h.message.as_deref(), Some("disk error"));
    }
}
