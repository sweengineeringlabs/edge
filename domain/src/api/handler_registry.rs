//! `HandlerRegistry` — thread-safe registry of [`Handler`] implementations
//! keyed by their stable id.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use super::handler::Handler;

/// Registry of [`Handler`] instances keyed by [`Handler::id`].
///
/// Concurrency: guarded by a `parking_lot::RwLock` — lookups proceed in
/// parallel while registration and deregistration are serialized.
pub struct HandlerRegistry<Req, Response>
where
    Req: Send + 'static,
    Response: Send + 'static,
{
    handlers: RwLock<HashMap<String, Arc<dyn Handler<Req, Response>>>>,
}

impl<Req, Response> HandlerRegistry<Req, Response>
where
    Req: Send + 'static,
    Response: Send + 'static,
{
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self { handlers: RwLock::new(HashMap::new()) }
    }

    /// Register a handler, replacing any existing entry with the same id.
    pub fn register(&self, handler: Arc<dyn Handler<Req, Response>>) {
        let id = handler.id().to_string();
        self.handlers.write().insert(id, handler);
    }

    /// Deregister the handler with the given id. Returns `true` if removed.
    pub fn deregister(&self, id: &str) -> bool {
        self.handlers.write().remove(id).is_some()
    }

    /// Look up a handler by id. Returns `None` if not registered.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Handler<Req, Response>>> {
        self.handlers.read().get(id).cloned()
    }

    /// Snapshot of registered handler ids. Order is unspecified.
    pub fn list_ids(&self) -> Vec<String> {
        self.handlers.read().keys().cloned().collect()
    }

    /// Number of currently registered handlers.
    pub fn len(&self) -> usize {
        self.handlers.read().len()
    }

    /// Whether the registry has no handlers.
    pub fn is_empty(&self) -> bool {
        self.handlers.read().is_empty()
    }
}

impl<Req, Response> Default for HandlerRegistry<Req, Response>
where
    Req: Send + 'static,
    Response: Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::error::HandlerError;
    use async_trait::async_trait;
    use std::any::Any;

    struct StubHandler { id: String }

    #[async_trait]
    impl Handler<String, String> for StubHandler {
        fn id(&self) -> &str { &self.id }
        fn pattern(&self) -> &str { "stub" }
        async fn execute(&self, req: String) -> Result<String, HandlerError> { Ok(req) }
        async fn health_check(&self) -> bool { true }
        fn as_any(&self) -> &dyn Any { self }
    }

    fn stub(id: &str) -> Arc<dyn Handler<String, String>> {
        Arc::new(StubHandler { id: id.to_string() })
    }

    #[test]
    fn test_register_and_get_returns_some() {
        let reg: HandlerRegistry<String, String> = HandlerRegistry::new();
        reg.register(stub("a"));
        assert!(reg.get("a").is_some());
    }

    #[test]
    fn test_get_missing_returns_none() {
        let reg: HandlerRegistry<String, String> = HandlerRegistry::new();
        assert!(reg.get("nope").is_none());
    }

    #[test]
    fn test_deregister_returns_true_when_present() {
        let reg: HandlerRegistry<String, String> = HandlerRegistry::new();
        reg.register(stub("a"));
        assert!(reg.deregister("a"));
        assert!(!reg.deregister("a"));
    }

    #[test]
    fn test_register_replaces_existing() {
        let reg: HandlerRegistry<String, String> = HandlerRegistry::new();
        reg.register(stub("a"));
        reg.register(stub("a"));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn test_list_ids_reports_all_registered() {
        let reg: HandlerRegistry<String, String> = HandlerRegistry::new();
        reg.register(stub("a"));
        reg.register(stub("b"));
        let mut ids = reg.list_ids();
        ids.sort();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_empty_registry_is_empty() {
        let reg: HandlerRegistry<String, String> = HandlerRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }
}
