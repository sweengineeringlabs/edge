//! HTTP request body types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP request body types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum HttpBody {
    Json(serde_json::Value),
    Raw(Vec<u8>),
    Form(HashMap<String, String>),
    Multipart(Vec<FormPart>),
}

/// A part of a multipart form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormPart {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_body_json_variant_holds_value() {
        let v = serde_json::json!({"key": "val"});
        let body = HttpBody::Json(v.clone());
        assert!(matches!(body, HttpBody::Json(_)));
    }

    #[test]
    fn test_http_body_raw_variant_holds_bytes() {
        let body = HttpBody::Raw(vec![1, 2, 3]);
        assert!(matches!(body, HttpBody::Raw(ref b) if b == &[1, 2, 3]));
    }
}
