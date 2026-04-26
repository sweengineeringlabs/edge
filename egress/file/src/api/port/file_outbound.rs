//! FileOutbound trait — writes to file storage.

use std::pin::Pin;

use futures::future::BoxFuture;
use futures::stream::Stream;
use thiserror::Error;

use crate::api::value_object::{FileInfo, ListOptions, ListResult, PresignedUrl, UploadOptions};

/// Error type for file outbound operations.
#[derive(Debug, Error)]
pub enum FileError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("io error: {0}")]
    IoError(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Result type for file outbound operations.
pub type FileResult<T> = Result<T, FileError>;

/// Outbound operations for file storage (write operations).
pub trait FileOutbound: Send + Sync {
    fn write(&self, path: &str, data: Vec<u8>, options: UploadOptions) -> BoxFuture<'_, FileResult<FileInfo>>;
    fn delete(&self, path: &str) -> BoxFuture<'_, FileResult<()>>;
    fn copy(&self, source: &str, destination: &str) -> BoxFuture<'_, FileResult<FileInfo>>;
    fn metadata(&self, path: &str) -> BoxFuture<'_, FileResult<FileInfo>>;
    fn list(&self, options: ListOptions) -> BoxFuture<'_, FileResult<ListResult>>;
    fn presigned_write_url(&self, path: &str, expires_in_secs: u64) -> BoxFuture<'_, FileResult<PresignedUrl>>;
    fn health_check(&self) -> BoxFuture<'_, FileResult<()>>;

    #[allow(clippy::type_complexity)]
    fn list_stream(
        &self,
        options: ListOptions,
    ) -> BoxFuture<'_, FileResult<Pin<Box<dyn Stream<Item = FileResult<FileInfo>> + Send + '_>>>>
    {
        Box::pin(async move {
            let result = self.list(options).await?;
            let stream: Pin<Box<dyn Stream<Item = FileResult<FileInfo>> + Send + '_>> =
                Box::pin(futures::stream::iter(result.files.into_iter().map(Ok)));
            Ok(stream)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_outbound_is_object_safe() {
        fn _assert_object_safe(_: &dyn FileOutbound) {}
    }
}
