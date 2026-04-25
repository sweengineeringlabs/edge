//! File metadata type.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata associated with a file in storage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileMetadata {
    pub content_type: Option<String>,
    pub content_encoding: Option<String>,
    pub cache_control: Option<String>,
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl FileMetadata {
    pub fn with_content_type(content_type: impl Into<String>) -> Self {
        Self { content_type: Some(content_type.into()), ..Default::default() }
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
