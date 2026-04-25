//! A database record as a key-value map.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A single database record represented as a JSON value map.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Record {
    pub fields: HashMap<String, Value>,
}

impl Record {
    pub fn new() -> Self { Self { fields: HashMap::new() } }

    pub fn set(mut self, key: impl Into<String>, value: Value) -> Self {
        self.fields.insert(key.into(), value); self
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: set
    #[test]
    fn test_set_inserts_field_into_record() {
        let r = Record::new().set("id", Value::String("abc".into()));
        assert_eq!(r.get("id"), Some(&Value::String("abc".into())));
    }

    /// @covers: get
    #[test]
    fn test_get_returns_none_for_missing_field() {
        let r = Record::new();
        assert!(r.get("missing").is_none());
    }
}
