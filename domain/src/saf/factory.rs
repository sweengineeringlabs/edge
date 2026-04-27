//! Factory functions for domain building blocks.

use std::sync::Arc;

use crate::api::handler_registry::HandlerRegistry;

/// Construct a fresh empty [`HandlerRegistry`].
///
/// Returned as `Arc<_>` because the registry is typically shared between
/// a `Job` impl and operator tooling that lists or mutates the handler set.
pub fn new_handler_registry<Req, Response>() -> Arc<HandlerRegistry<Req, Response>>
where
    Req: Send + 'static,
    Response: Send + 'static,
{
    Arc::new(HandlerRegistry::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_returns_empty_registry() {
        let reg: Arc<HandlerRegistry<String, String>> = new_handler_registry();
        assert!(reg.is_empty());
    }
}
