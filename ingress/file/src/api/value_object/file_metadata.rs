//! File metadata type.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata associated with a file in storage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileMetadata {
    /// MIME type to set on upload.
    pub content_type: Option<String>,
    /// Content-Encoding header value.
    pub content_encoding: Option<String>,
    /// Cache-Control header value.
    pub cache_control: Option<String>,
    /// Arbitrary key/value metadata for the backend.
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl FileMetadata {
    /// Create metadata with a content type.
    pub fn with_content_type(content_type: impl Into<String>) -> Self {
        Self { content_type: Some(content_type.into()), ..Default::default() }
    }

    /// Add a custom metadata entry.
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into()); self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: with_content_type
    #[test]
    fn test_with_content_type_sets_content_type() {
        let meta = FileMetadata::with_content_type("application/json");
        assert_eq!(meta.content_type, Some("application/json".to_string()));
    }

    /// @covers: with_custom
    #[test]
    fn test_with_custom_inserts_key_value_pair() {
        let meta = FileMetadata::default().with_custom("k", "v");
        assert_eq!(meta.custom.get("k"), Some(&"v".to_string()));
    }
}
