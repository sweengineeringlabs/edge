//! HTTP client/server configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_max_response_bytes() -> Option<usize> { Some(10 * 1024 * 1024) }

/// HTTP client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpConfig {
    /// Base URL prepended to all request URLs.
    pub base_url: Option<String>,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Headers added to every request.
    #[serde(default)]
    pub default_headers: HashMap<String, String>,
    /// Whether to follow HTTP redirects.
    pub follow_redirects: bool,
    /// Maximum number of redirects to follow.
    pub max_redirects: u32,
    /// User-Agent header value.
    pub user_agent: Option<String>,
    /// Maximum response body size to accept.
    #[serde(default = "default_max_response_bytes")]
    pub max_response_bytes: Option<usize>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            timeout_secs: 30,
            connect_timeout_secs: 10,
            max_retries: 3,
            default_headers: HashMap::new(),
            follow_redirects: true,
            max_redirects: 10,
            user_agent: Some("swe-edge/0.1.0".to_string()),
            max_response_bytes: default_max_response_bytes(),
        }
    }
}

impl HttpConfig {
    /// Create a config with a base URL.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self { base_url: Some(base_url.into()), ..Default::default() }
    }

    /// Add a default header to the config.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.insert(name.into(), value.into());
        self
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: with_base_url
    #[test]
    fn test_with_base_url_sets_base_url() {
        let cfg = HttpConfig::with_base_url("http://x.com");
        assert_eq!(cfg.base_url, Some("http://x.com".to_string()));
    }

    /// @covers: with_header
    #[test]
    fn test_with_header_inserts_default_header() {
        let cfg = HttpConfig::default().with_header("X-Key", "val");
        assert_eq!(cfg.default_headers.get("X-Key"), Some(&"val".to_string()));
    }

    /// @covers: with_timeout
    #[test]
    fn test_with_timeout_sets_timeout_secs() {
        let cfg = HttpConfig::default().with_timeout(60);
        assert_eq!(cfg.timeout_secs, 60);
    }
}
