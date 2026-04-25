//! Gateway provider traits for stateless and stateful implementations.

use std::future::Future;
use std::sync::Arc;

use crate::api::egress_error::EgressResult;

/// Marker trait for stateless gateway providers.
pub trait StatelessProvider: Default + Clone + Send + Sync + 'static {}

/// Marker trait for stateful gateway providers.
pub trait StatefulProvider: Send + Sync + 'static {}

/// Trait for providers that support lazy service initialization.
pub trait LazyInit<S: ?Sized>: Send + Sync {
    fn get_service(&self) -> impl Future<Output = EgressResult<Arc<S>>> + Send;
    fn is_initialized(&self) -> bool;
    fn reset(&self);
}

/// Trait for providers that support parameterized initialization.
pub trait LazyInitWithConfig<S: ?Sized, C>: Send + Sync {
    fn get_service_with_config(&self, config: &C) -> impl Future<Output = EgressResult<Arc<S>>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Clone)]
    struct TestStateless;
    impl StatelessProvider for TestStateless {}

    struct TestStateful;
    impl StatefulProvider for TestStateful {}

    #[test]
    fn test_stateless_provider_can_be_default_and_cloned() {
        let p = TestStateless::default();
        let _ = p.clone();
    }

    #[test]
    fn test_stateful_provider_can_be_constructed() {
        let _ = TestStateful;
    }
}
