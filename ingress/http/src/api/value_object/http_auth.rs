//! HTTP authentication types.

use serde::{Deserialize, Serialize};

/// Authentication method for HTTP requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HttpAuth {
    /// No authentication.
    #[default]
    None,
    /// Bearer token authentication.
    Bearer {
        /// The bearer token value.
        token: String,
    },
    /// HTTP Basic authentication.
    Basic {
        /// Basic auth username.
        username: String,
        /// Basic auth password.
        password: String,
    },
    /// API key authentication via a custom header.
    ApiKey {
        /// Header name for the API key.
        header: String,
        /// API key value.
        key: String,
    },
}

impl HttpAuth {
    /// Create Bearer token auth.
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer { token: token.into() }
    }

    /// Create Basic auth with username and password.
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Basic { username: username.into(), password: password.into() }
    }

    /// Create API key auth via a custom header name.
    pub fn api_key(header: impl Into<String>, key: impl Into<String>) -> Self {
        Self::ApiKey { header: header.into(), key: key.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: bearer
    #[test]
    fn test_bearer_creates_bearer_token_auth() {
        let auth = HttpAuth::bearer("tok");
        assert!(matches!(auth, HttpAuth::Bearer { ref token } if token == "tok"));
    }

    /// @covers: basic
    #[test]
    fn test_basic_creates_basic_auth_with_credentials() {
        let auth = HttpAuth::basic("user", "pass");
        assert!(matches!(auth, HttpAuth::Basic { ref username, .. } if username == "user"));
    }

    /// @covers: api_key
    #[test]
    fn test_api_key_creates_api_key_auth() {
        let auth = HttpAuth::api_key("X-Api-Key", "secret");
        assert!(matches!(auth, HttpAuth::ApiKey { ref header, .. } if header == "X-Api-Key"));
    }
}
