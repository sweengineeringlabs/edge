//! FileOutbound trait — writes to file storage.

use std::pin::Pin;

use futures::future::BoxFuture;
use futures::stream::Stream;

use crate::api::egress_error::EgressResult;
use crate::api::file::{FileInfo, ListOptions, ListResult, PresignedUrl, UploadOptions};
use crate::api::health_check::HealthCheck;

/// Outbound operations for file storage (write operations).
pub trait FileOutbound: Send + Sync {
    fn write(&self, path: &str, data: Vec<u8>, options: UploadOptions) -> BoxFuture<'_, EgressResult<FileInfo>>;
    fn delete(&self, path: &str) -> BoxFuture<'_, EgressResult<()>>;
    fn copy(&self, source: &str, destination: &str) -> BoxFuture<'_, EgressResult<FileInfo>>;
    fn metadata(&self, path: &str) -> BoxFuture<'_, EgressResult<FileInfo>>;
    fn list(&self, options: ListOptions) -> BoxFuture<'_, EgressResult<ListResult>>;
    fn presigned_write_url(&self, path: &str, expires_in_secs: u64) -> BoxFuture<'_, EgressResult<PresignedUrl>>;
    fn health_check(&self) -> BoxFuture<'_, EgressResult<HealthCheck>>;

    #[allow(clippy::type_complexity)]
    fn list_stream(
        &self,
        options: ListOptions,
    ) -> BoxFuture<'_, EgressResult<Pin<Box<dyn Stream<Item = EgressResult<FileInfo>> + Send + '_>>>>
    {
        Box::pin(async move {
            let result = self.list(options).await?;
            let stream: Pin<Box<dyn Stream<Item = EgressResult<FileInfo>> + Send + '_>> =
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
