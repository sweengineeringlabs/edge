//! File metadata for upload operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata attached to a file on upload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileMetadata {
    pub content_type: Option<String>,
    pub content_encoding: Option<String>,
    pub cache_control: Option<String>,
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl FileMetadata {
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into()); self
    }

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
        let m = FileMetadata::default().with_content_type("image/png");
        assert_eq!(m.content_type, Some("image/png".to_string()));
    }

    /// @covers: with_custom
    #[test]
    fn test_with_custom_inserts_key_value() {
        let m = FileMetadata::default().with_custom("x-owner", "svc-a");
        assert_eq!(m.custom.get("x-owner"), Some(&"svc-a".to_string()));
    }
}
