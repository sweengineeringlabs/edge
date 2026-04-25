//! File storage configuration.

use serde::{Deserialize, Serialize};

/// Storage backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStorageType {
    Local,
    S3,
    Gcs,
    Azure,
    Memory,
}

/// Configuration for a file storage backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStorageConfig {
    pub storage_type: FileStorageType,
    pub root: Option<String>,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
}

impl FileStorageConfig {
    pub fn local(root: impl Into<String>) -> Self {
        Self { storage_type: FileStorageType::Local, root: Some(root.into()), bucket: None, region: None, endpoint: None }
    }

    pub fn memory() -> Self {
        Self { storage_type: FileStorageType::Memory, root: None, bucket: None, region: None, endpoint: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: local
    #[test]
    fn test_local_creates_local_storage_config() {
        let cfg = FileStorageConfig::local("/tmp/files");
        assert_eq!(cfg.storage_type, FileStorageType::Local);
        assert_eq!(cfg.root, Some("/tmp/files".to_string()));
    }

    /// @covers: memory
    #[test]
    fn test_memory_creates_memory_storage_config() {
        let cfg = FileStorageConfig::memory();
        assert_eq!(cfg.storage_type, FileStorageType::Memory);
    }
}
