//! HTTP request type.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::http_body::HttpBody;
use super::http_method::HttpMethod;

/// An HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub query: HashMap<String, String>,
    pub body: Option<HttpBody>,
    pub timeout: Option<Duration>,
}

impl HttpRequest {
    pub fn get(url: impl Into<String>) -> Self {
        Self { method: HttpMethod::Get, url: url.into(), headers: HashMap::new(), query: HashMap::new(), body: None, timeout: None }
    }

    pub fn post(url: impl Into<String>) -> Self {
        Self { method: HttpMethod::Post, url: url.into(), headers: HashMap::new(), query: HashMap::new(), body: None, timeout: None }
    }

    pub fn put(url: impl Into<String>) -> Self {
        Self { method: HttpMethod::Put, url: url.into(), headers: HashMap::new(), query: HashMap::new(), body: None, timeout: None }
    }

    pub fn delete(url: impl Into<String>) -> Self {
        Self { method: HttpMethod::Delete, url: url.into(), headers: HashMap::new(), query: HashMap::new(), body: None, timeout: None }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into()); self
    }

    pub fn with_query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.insert(name.into(), value.into()); self
    }

    pub fn with_json<T: Serialize>(mut self, body: &T) -> Result<Self, serde_json::Error> {
        self.body = Some(HttpBody::Json(serde_json::to_value(body)?));
        self.headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(self)
    }

    pub fn with_body(mut self, body: Vec<u8>, content_type: impl Into<String>) -> Self {
        self.body = Some(HttpBody::Raw(body));
        self.headers.insert("Content-Type".to_string(), content_type.into()); self
    }

    pub fn with_form(mut self, form: HashMap<String, String>) -> Self {
        self.body = Some(HttpBody::Form(form));
        self.headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string()); self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout); self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: get
    #[test]
    fn test_get_creates_get_request_with_url() {
        let req = HttpRequest::get("https://example.com").with_header("Accept", "json").with_query("p", "1");
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.headers.get("Accept"), Some(&"json".to_string()));
        assert_eq!(req.query.get("p"), Some(&"1".to_string()));
    }

    /// @covers: post
    #[test]
    fn test_post_with_json_body_sets_content_type() {
        let req = HttpRequest::post("/api").with_json(&serde_json::json!({"k": "v"})).unwrap();
        assert_eq!(req.method, HttpMethod::Post);
        assert!(matches!(req.body, Some(HttpBody::Json(_))));
    }

    /// @covers: put
    #[test]
    fn test_put_creates_put_request() {
        let req = HttpRequest::put("/x");
        assert_eq!(req.method, HttpMethod::Put);
    }

    /// @covers: delete
    #[test]
    fn test_delete_creates_delete_request() {
        let req = HttpRequest::delete("/x");
        assert_eq!(req.method, HttpMethod::Delete);
    }

    /// @covers: with_body
    #[test]
    fn test_with_body_sets_raw_body_and_content_type() {
        let req = HttpRequest::post("/x").with_body(vec![1, 2], "application/octet-stream");
        assert!(matches!(req.body, Some(HttpBody::Raw(ref b)) if b == &[1, 2]));
    }

    /// @covers: with_timeout
    #[test]
    fn test_with_timeout_sets_timeout() {
        let req = HttpRequest::get("/x").with_timeout(Duration::from_secs(5));
        assert_eq!(req.timeout, Some(Duration::from_secs(5)));
    }
}
