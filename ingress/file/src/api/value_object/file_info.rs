//! File information type.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata about a file in storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Path or key within the storage backend.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// MIME type when known.
    pub content_type: Option<String>,
    /// Timestamp of the last write.
    pub last_modified: DateTime<Utc>,
    /// Timestamp of creation when available.
    pub created_at: Option<DateTime<Utc>>,
    /// ETag for conditional requests.
    pub etag: Option<String>,
    /// `true` for a directory or prefix entry.
    pub is_directory: bool,
    /// Backend-specific metadata key/value pairs.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl FileInfo {
    /// Create a file entry with `path` and `size`.
    pub fn new(path: impl Into<String>, size: u64) -> Self {
        Self { path: path.into(), size, content_type: None, last_modified: Utc::now(), created_at: None, etag: None, is_directory: false, metadata: HashMap::new() }
    }

    /// Create a directory entry at `path`.
    pub fn directory(path: impl Into<String>) -> Self {
        Self { path: path.into(), size: 0, content_type: None, last_modified: Utc::now(), created_at: None, etag: None, is_directory: true, metadata: HashMap::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_info_new_creates_non_directory_entry() {
        let info = FileInfo::new("docs/readme.md", 1024);
        assert_eq!(info.path, "docs/readme.md");
        assert_eq!(info.size, 1024);
        assert!(!info.is_directory);
    }

    /// @covers: directory
    #[test]
    fn test_directory_creates_directory_entry_with_zero_size() {
        let info = FileInfo::directory("docs/");
        assert!(info.is_directory);
        assert_eq!(info.size, 0);
    }
}
