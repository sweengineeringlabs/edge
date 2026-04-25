//! HTTP authentication types.

use serde::{Deserialize, Serialize};

/// HTTP authentication scheme.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpAuth {
    None,
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { header: String, key: String },
}

impl HttpAuth {
    pub fn bearer(token: impl Into<String>) -> Self {
        HttpAuth::Bearer { token: token.into() }
    }

    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        HttpAuth::Basic { username: username.into(), password: password.into() }
    }

    pub fn api_key(header: impl Into<String>, key: impl Into<String>) -> Self {
        HttpAuth::ApiKey { header: header.into(), key: key.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: bearer
    #[test]
    fn test_bearer_creates_bearer_auth_with_token() {
        let auth = HttpAuth::bearer("tok_abc");
        assert!(matches!(auth, HttpAuth::Bearer { token } if token == "tok_abc"));
    }

    /// @covers: basic
    #[test]
    fn test_basic_creates_basic_auth_with_credentials() {
        let auth = HttpAuth::basic("user", "pass");
        assert!(matches!(auth, HttpAuth::Basic { username, .. } if username == "user"));
    }

    /// @covers: api_key
    #[test]
    fn test_api_key_creates_api_key_auth() {
        let auth = HttpAuth::api_key("X-Api-Key", "secret");
        assert!(matches!(auth, HttpAuth::ApiKey { header, .. } if header == "X-Api-Key"));
    }
}
