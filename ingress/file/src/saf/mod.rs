//! SAF layer — file inbound public facade.

pub use crate::api::value_object::{FileInfo, FileMetadata, FileStorageConfig, FileStorageType, ListOptions, ListResult, PresignedUrl};
pub use crate::api::port::file_inbound::{FileInbound, FileInboundError, FileInboundResult, FileHealthCheck};
pub use crate::core::file::LocalFileSource;

/// Construct a local-filesystem FileInbound from the given base path.
pub fn local_file_source(base_path: impl Into<String>) -> LocalFileSource {
    LocalFileSource::new(FileStorageConfig::local(base_path))
}
