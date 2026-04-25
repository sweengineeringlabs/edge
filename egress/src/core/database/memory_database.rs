//! In-memory database egress adapter — for testing and development.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::api::database::DatabaseGateway;
use crate::api::database::DatabaseRead;
use crate::api::database::DatabaseWrite;
use crate::api::egress_error::EgressError;

/// Thread-safe in-memory key/value store.
pub(crate) struct MemoryDatabase {
    store: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl MemoryDatabase {
    pub(crate) fn new() -> Self {
        Self { store: Mutex::new(BTreeMap::new()) }
    }
}

impl DatabaseRead for MemoryDatabase {
    fn describe(&self) -> &'static str {
        "swe_edge_egress::memory_database"
    }

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, EgressError> {
        Ok(self.store.lock().unwrap().get(key).cloned())
    }

    fn list(&self, prefix: Option<&str>) -> Result<Vec<String>, EgressError> {
        let store = self.store.lock().unwrap();
        let keys = match prefix {
            Some(p) => store.keys().filter(|k| k.starts_with(p)).cloned().collect(),
            None => store.keys().cloned().collect(),
        };
        Ok(keys)
    }
}

impl DatabaseWrite for MemoryDatabase {
    fn describe(&self) -> &'static str {
        "swe_edge_egress::memory_database"
    }

    fn put(&self, key: &str, value: &[u8]) -> Result<(), EgressError> {
        self.store.lock().unwrap().insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), EgressError> {
        self.store.lock().unwrap().remove(key);
        Ok(())
    }
}

impl DatabaseGateway for MemoryDatabase {}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> MemoryDatabase { MemoryDatabase::new() }

    #[test]
    fn test_new_creates_empty_store() {
        let d = db();
        assert_eq!(d.list(None).unwrap().len(), 0);
    }

    #[test]
    fn test_put_stores_value_retrievable_by_get() {
        let d = db();
        d.put("k", b"v").unwrap();
        assert_eq!(d.get("k").unwrap(), Some(b"v".to_vec()));
    }

    #[test]
    fn test_delete_removes_existing_key() {
        let d = db();
        d.put("k", b"v").unwrap();
        d.delete("k").unwrap();
        assert_eq!(d.get("k").unwrap(), None);
    }

    #[test]
    fn test_list_with_prefix_filters_correctly() {
        let d = db();
        d.put("foo/1", b"a").unwrap();
        d.put("foo/2", b"b").unwrap();
        d.put("bar/1", b"c").unwrap();
        let keys = d.list(Some("foo/")).unwrap();
        assert_eq!(keys.len(), 2);
    }
}
