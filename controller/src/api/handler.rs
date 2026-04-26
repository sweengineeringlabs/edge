//! Handler trait — **Handlers** concern of the 5-Concern Controller pattern.
//!
//! A single execution unit. Every agent/service/VM in a Controller implements
//! `Handler`, providing a uniform contract that routing and registries can
//! dispatch to without knowing the domain specifics.

use std::any::Any;

use async_trait::async_trait;

use super::error::HandlerError;

/// A single execution unit that processes a request and returns a response.
///
/// Implementations wrap a concrete pattern (ReAct, CoT, direct call, etc.) or a
/// specific service (auth, authz, VM lifecycle).
///
/// `as_any` enables safe downcasting when a caller needs concrete access.
#[async_trait]
pub trait Handler<Req, Resp>: Send + Sync
where
    Req: Send + 'static,
    Resp: Send + 'static,
{
    /// Stable identifier — used as the lookup key in `HandlerRegistry`.
    fn id(&self) -> &str;

    /// Human-readable pattern or service name (e.g. "ReAct", "AuthN", "KVM").
    fn pattern(&self) -> &str;

    /// Execute the handler with the given request.
    async fn execute(&self, req: Req) -> Result<Resp, HandlerError>;

    /// Probe whether the handler is healthy and responsive.
    async fn health_check(&self) -> bool;

    /// Downcast hook for concrete access from tests or administrative tools.
    fn as_any(&self) -> &dyn Any;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_trait_is_object_safe() {
        fn _accept(_h: &dyn Handler<String, String>) {}
    }
}
