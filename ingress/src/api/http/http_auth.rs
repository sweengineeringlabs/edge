//! HTTP authentication types.

use serde::{Deserialize, Serialize};

/// Authentication method for HTTP requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HttpAuth {
    #[default]
    None,
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { header: String, key: String },
}

impl HttpAuth {
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer { token: token.into() }
    }

    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Basic { username: username.into(), password: password.into() }
    }

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
