//! File storage backend configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// File storage backend type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStorageType {
    /// Local filesystem.
    #[default]
    Local,
    /// Amazon S3 or S3-compatible object store.
    S3,
    /// Google Cloud Storage.
    Gcs,
    /// Azure Blob Storage.
    Azure,
    /// In-memory store (for testing).
    Memory,
}

/// Configuration for file storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FileStorageConfig {
    /// Backend type.
    pub storage_type: FileStorageType,
    /// Base path or bucket name.
    pub base_path: String,
    /// Cloud region when applicable.
    pub region: Option<String>,
    /// Access key (not serialized).
    #[serde(skip_serializing)]
    pub access_key: Option<String>,
    /// Secret key (not serialized).
    #[serde(skip_serializing)]
    pub secret_key: Option<String>,
    /// Custom endpoint for S3-compatible backends.
    pub endpoint: Option<String>,
    /// Extra backend-specific options.
    #[serde(default)]
    pub options: HashMap<String, String>,
}

impl Default for FileStorageConfig {
    fn default() -> Self {
        Self { storage_type: FileStorageType::Local, base_path: ".".to_string(), region: None, access_key: None, secret_key: None, endpoint: None, options: HashMap::new() }
    }
}

impl FileStorageConfig {
    /// Create a local filesystem config rooted at `base_path`.
    pub fn local(base_path: impl Into<String>) -> Self {
        Self { storage_type: FileStorageType::Local, base_path: base_path.into(), ..Default::default() }
    }

    /// Create an in-memory config (for testing).
    pub fn memory() -> Self {
        Self { storage_type: FileStorageType::Memory, base_path: "/".to_string(), ..Default::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: local
    #[test]
    fn test_local_creates_local_storage_config() {
        let cfg = FileStorageConfig::local("/data");
        assert_eq!(cfg.storage_type, FileStorageType::Local);
        assert_eq!(cfg.base_path, "/data");
    }

    /// @covers: memory
    #[test]
    fn test_memory_creates_in_memory_storage_config() {
        let cfg = FileStorageConfig::memory();
        assert_eq!(cfg.storage_type, FileStorageType::Memory);
    }
}
