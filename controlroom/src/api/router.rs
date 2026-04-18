//! Router trait — **Routing** concern of the 5-Concern ControlRoom pattern.
//!
//! Classifies an inbound input and returns a domain-defined `Intent` that
//! downstream logic (typically a `HandlerRegistry`) uses to pick the right
//! `Handler`.

use async_trait::async_trait;

use super::error::RoutingError;

/// Classifies input into a domain-specific intent.
///
/// | Domain | Intent |
/// |--------|--------|
/// | llmboot | `AgentIntent` (naming target agent / pattern) |
/// | security/iam | `ServiceIntent` (auth vs authz vs iam) |
/// | vmisolate | *(absent — VM ops are direct, no routing)* |
#[async_trait]
pub trait Router<Intent>: Send + Sync
where
    Intent: Send + 'static,
{
    /// Classify the input string and return the resolved intent.
    async fn route(&self, input: &str) -> Result<Intent, RoutingError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_trait_is_object_safe() {
        fn _accept(_r: &dyn Router<String>) {}
    }
}
