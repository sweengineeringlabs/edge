//! File storage backend configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// File storage backend type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStorageType {
    #[default]
    Local,
    S3,
    Gcs,
    Azure,
    Memory,
}

/// Configuration for file storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FileStorageConfig {
    pub storage_type: FileStorageType,
    pub base_path: String,
    pub region: Option<String>,
    #[serde(skip_serializing)]
    pub access_key: Option<String>,
    #[serde(skip_serializing)]
    pub secret_key: Option<String>,
    pub endpoint: Option<String>,
    #[serde(default)]
    pub options: HashMap<String, String>,
}

impl Default for FileStorageConfig {
    fn default() -> Self {
        Self { storage_type: FileStorageType::Local, base_path: ".".to_string(), region: None, access_key: None, secret_key: None, endpoint: None, options: HashMap::new() }
    }
}

impl FileStorageConfig {
    pub fn local(base_path: impl Into<String>) -> Self {
        Self { storage_type: FileStorageType::Local, base_path: base_path.into(), ..Default::default() }
    }

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
