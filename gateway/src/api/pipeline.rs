//! Pipeline and Router traits — the public contract for request/response
//! processing chains.
//!
//! The runtime implementations (`DefaultPipeline`, `PipelineRouter`) live in
//! `crate::core::pipeline` as `pub(crate)`. Consumers obtain a pipeline or
//! router via the factories in `saf/`, which return `impl Pipeline` /
//! `impl Router` so the concrete core types are never named.
//!
//! Rules enforced by this placement:
//!   - rule 50: the default impls stay `pub(crate)` in core/.
//!   - rule 159: saf/ public signatures take / return these api/ traits.
//!   - rule 160: all public trait declarations live under api/.

use async_trait::async_trait;

use crate::api::types::GatewayError;

/// Router trait — dispatches a request to produce a response.
///
/// Generic over `Req`, `Resp`, `Err`.
#[async_trait]
pub trait Router<
    Req: Send + Sync + 'static = serde_json::Value,
    Resp: Send + Sync + 'static = serde_json::Value,
    Err: Send + Sync + 'static = GatewayError,
>: Send + Sync {
    /// Dispatch a request and produce a response.
    async fn dispatch(&self, request: &Req) -> Result<Resp, Err>;
}

/// Pipeline — executes a request through an ordered chain of stages.
///
/// Generic over `Req`, `Resp`, `Err`.
///
/// The default implementation, obtained from `saf::default_pipeline`,
/// composes pre-middleware, a router, and post-middleware with
/// short-circuit support. Implement this trait directly for custom
/// execution strategies (e.g. metered, cached, or fan-out pipelines).
#[async_trait]
pub trait Pipeline<
    Req: Send + Sync + 'static = serde_json::Value,
    Resp: Send + Sync + 'static = serde_json::Value,
    Err: Send + Sync + 'static = GatewayError,
>: Send + Sync {
    /// Execute the pipeline end-to-end: pre-middleware, router, post-middleware.
    async fn execute(&self, request: Req) -> Result<Resp, Err>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_is_object_safe() {
        fn _accepts(_r: &dyn Router) {}
    }

    #[test]
    fn test_pipeline_is_object_safe() {
        fn _accepts(_p: &dyn Pipeline) {}
    }
}
