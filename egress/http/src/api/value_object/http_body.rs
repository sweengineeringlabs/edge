//! HTTP body types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A multipart form part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormPart {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Vec<u8>,
}

/// HTTP request body variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpBody {
    Json(serde_json::Value),
    Raw(Vec<u8>),
    Form(HashMap<String, String>),
    Multipart(Vec<FormPart>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_body_json_holds_value() {
        let body = HttpBody::Json(serde_json::json!({"k": "v"}));
        assert!(matches!(body, HttpBody::Json(_)));
    }

    #[test]
    fn test_http_body_raw_holds_bytes() {
        let body = HttpBody::Raw(vec![1, 2, 3]);
        assert!(matches!(body, HttpBody::Raw(ref b) if b == &[1, 2, 3]));
    }
}
