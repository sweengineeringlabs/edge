//! Presigned URL for temporary file access.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A presigned URL for file access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignedUrl {
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub method: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presigned_url_holds_url_and_method() {
        let p = PresignedUrl { url: "https://example.com/file".into(), expires_at: Utc::now(), method: "GET".into() };
        assert_eq!(p.method, "GET");
        assert!(!p.url.is_empty());
    }
}
