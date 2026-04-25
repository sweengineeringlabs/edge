//! File inbound trait — reads from file storage.

use std::pin::Pin;

use futures::future::BoxFuture;
use futures::stream::Stream;

use crate::api::file::{FileInfo, ListOptions, ListResult, PresignedUrl};
use crate::api::health_check::HealthCheck;
use crate::api::ingress_error::IngressResult;

/// Inbound operations for file storage (read operations).
pub trait FileInbound: Send + Sync {
    fn read(&self, path: &str) -> BoxFuture<'_, IngressResult<Vec<u8>>>;
    fn metadata(&self, path: &str) -> BoxFuture<'_, IngressResult<FileInfo>>;
    fn list(&self, options: ListOptions) -> BoxFuture<'_, IngressResult<ListResult>>;
    fn exists(&self, path: &str) -> BoxFuture<'_, IngressResult<bool>>;
    fn presigned_read_url(&self, path: &str, expires_in_secs: u64) -> BoxFuture<'_, IngressResult<PresignedUrl>>;
    fn health_check(&self) -> BoxFuture<'_, IngressResult<HealthCheck>>;

    #[allow(clippy::type_complexity)]
    fn list_stream(
        &self,
        options: ListOptions,
    ) -> BoxFuture<'_, IngressResult<Pin<Box<dyn Stream<Item = IngressResult<FileInfo>> + Send + '_>>>>
    {
        Box::pin(async move {
            let result = self.list(options).await?;
            let stream: Pin<Box<dyn Stream<Item = IngressResult<FileInfo>> + Send + '_>> =
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
}
