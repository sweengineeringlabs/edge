//! Closure-backed Router implementation.

use async_trait::async_trait;
use futures::future::BoxFuture;

use crate::api::pipeline::Router;
use crate::api::types::GatewayError;

/// Async closure-based router.
///
/// Use this when your dispatch logic is async — e.g. calling an external service.
/// The handler captures whatever dependencies it needs at construction time,
/// keeping the router free of domain knowledge.
pub(crate) struct PipelineRouter<
    F,
    Req = serde_json::Value,
    Resp = serde_json::Value,
    Err = GatewayError,
>
where
    F: for<'a> Fn(&'a Req) -> BoxFuture<'a, Result<Resp, Err>> + Send + Sync,
{
    handler: F,
    _phantom: std::marker::PhantomData<(Req, Resp, Err)>,
}

impl<F, Req, Resp, Err> PipelineRouter<F, Req, Resp, Err>
where
    F: for<'a> Fn(&'a Req) -> BoxFuture<'a, Result<Resp, Err>> + Send + Sync,
{
    pub(crate) fn new(handler: F) -> Self {
        Self { handler, _phantom: std::marker::PhantomData }
    }
}

#[async_trait]
impl<F, Req, Resp, Err> Router<Req, Resp, Err> for PipelineRouter<F, Req, Resp, Err>
where
    F: for<'a> Fn(&'a Req) -> BoxFuture<'a, Result<Resp, Err>> + Send + Sync,
    Req: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
    Err: Send + Sync + 'static,
{
    async fn dispatch(&self, request: &Req) -> Result<Resp, Err> {
        (self.handler)(request).await
    }
}
