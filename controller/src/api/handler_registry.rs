//! `HandlerRegistry` — thread-safe registry of `Handler` implementations
//! keyed by their stable id.
//!
//! Domain-specific `Job` implementations typically own a `HandlerRegistry`
//! and resolve a `Router::route` result to a `Handler` via `get`.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use super::error::JobError;
use super::handler::Handler;

/// Registry of `Handler` instances keyed by `Handler::id`.
///
/// Concurrency: guarded by a `parking_lot::RwLock`, so lookups can proceed
/// in parallel while registration/deregistration are serialized.
pub struct HandlerRegistry<Req, Resp>
where
    Req: Send + 'static,
    Resp: Send + 'static,
{
    handlers: RwLock<HashMap<String, Arc<dyn Handler<Req, Resp>>>>,
}

impl<Req, Resp> HandlerRegistry<Req, Resp>
where
    Req: Send + 'static,
    Resp: Send + 'static,
{
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self { handlers: RwLock::new(HashMap::new()) }
    }

    /// Register a handler, replacing any existing entry with the same id.
    pub fn register(&self, handler: Arc<dyn Handler<Req, Resp>>) {
        let id = handler.id().to_string();
        self.handlers.write().insert(id, handler);
    }

    /// Deregister the handler with the given id. Returns `true` if removed.
    pub fn deregister(&self, id: &str) -> bool {
        self.handlers.write().remove(id).is_some()
    }

    /// Look up a handler by id, returning `JobError::HandlerUnavailable` if
    /// not found. Cheap: acquires a read lock and clones an `Arc`.
    pub fn get(&self, id: &str) -> Result<Arc<dyn Handler<Req, Resp>>, JobError> {
        self.handlers
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| JobError::HandlerUnavailable(id.to_string()))
    }

    /// Snapshot of the registered handler ids. Order is unspecified.
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

impl<Req, Resp> Default for HandlerRegistry<Req, Resp>
where
    Req: Send + 'static,
    Resp: Send + 'static,
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

    struct StubHandler {
        id: String,
    }

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
    fn test_register_and_get() {
        let reg: HandlerRegistry<String, String> = HandlerRegistry::new();
        reg.register(stub("a"));
        assert!(reg.get("a").is_ok());
    }

    #[test]
    fn test_get_missing_returns_handler_unavailable() {
        let reg: HandlerRegistry<String, String> = HandlerRegistry::new();
        match reg.get("nope") {
            Err(JobError::HandlerUnavailable(id)) => assert_eq!(id, "nope"),
            Err(e) => panic!("wrong error variant: {:?}", e),
            Ok(_) => panic!("expected Err, got Ok"),
        }
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
    fn test_list_ids_reports_all() {
        let reg: HandlerRegistry<String, String> = HandlerRegistry::new();
        reg.register(stub("a"));
        reg.register(stub("b"));
        let mut ids = reg.list_ids();
        ids.sort();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_empty_registry() {
        let reg: HandlerRegistry<String, String> = HandlerRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }
}
