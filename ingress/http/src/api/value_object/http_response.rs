//! HTTP response type.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response body bytes.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Create a response with `status` and `body`.
    pub fn new(status: u16, body: Vec<u8>) -> Self {
        Self { status, headers: HashMap::new(), body }
    }

    /// Returns `true` for 2xx status codes.
    pub fn is_success(&self) -> bool { (200..300).contains(&self.status) }

    /// Returns `true` for 4xx status codes.
    pub fn is_client_error(&self) -> bool { (400..500).contains(&self.status) }

    /// Returns `true` for 5xx status codes.
    pub fn is_server_error(&self) -> bool { (500..600).contains(&self.status) }

    /// Deserialise the body as JSON.
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    /// Decode the body as a UTF-8 string.
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body.clone())
    }

    /// Look up a response header (case-insensitive fallback to lowercase).
    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get(name).or_else(|| self.headers.get(&name.to_lowercase()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: is_success
    #[test]
    fn test_is_success_returns_true_for_2xx_status() {
        assert!(HttpResponse::new(200, vec![]).is_success());
        assert!(HttpResponse::new(299, vec![]).is_success());
        assert!(!HttpResponse::new(400, vec![]).is_success());
    }

    /// @covers: is_client_error
    #[test]
    fn test_is_client_error_returns_true_for_4xx_status() {
        assert!(HttpResponse::new(404, vec![]).is_client_error());
        assert!(!HttpResponse::new(200, vec![]).is_client_error());
    }

    /// @covers: is_server_error
    #[test]
    fn test_is_server_error_returns_true_for_5xx_status() {
        assert!(HttpResponse::new(500, vec![]).is_server_error());
        assert!(!HttpResponse::new(200, vec![]).is_server_error());
    }

    /// @covers: text
    #[test]
    fn test_text_returns_utf8_string_from_body() {
        let resp = HttpResponse::new(200, b"hello".to_vec());
        assert_eq!(resp.text().unwrap(), "hello");
    }

    /// @covers: header
    #[test]
    fn test_header_returns_value_for_known_header() {
        let mut resp = HttpResponse::new(200, vec![]);
        resp.headers.insert("Content-Type".to_string(), "text/html".to_string());
        assert_eq!(resp.header("Content-Type"), Some(&"text/html".to_string()));
        assert!(resp.header("X-Missing").is_none());
    }

    /// @covers: json
    #[test]
    fn test_json_parses_body_as_json_value() {
        let data = serde_json::json!({"name": "test"});
        let resp = HttpResponse::new(200, serde_json::to_vec(&data).unwrap());
        let parsed: serde_json::Value = resp.json().unwrap();
        assert_eq!(parsed["name"], "test");
    }
}
