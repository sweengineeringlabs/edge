//! Middleware traits for request/response pipeline processing.

use async_trait::async_trait;
use crate::api::ingress_error::IngressError;

/// Action returned by request middleware.
pub enum MiddlewareAction<Req, Resp> {
    /// Pass the (possibly modified) request to the next stage.
    Continue(Req),
    /// Skip remaining stages and return this response immediately.
    ShortCircuit(Resp),
}

/// Middleware that intercepts inbound requests.
#[async_trait]
pub trait RequestMiddleware<
    Req: Send + Sync + 'static = serde_json::Value,
    Err: Send + Sync + 'static = IngressError,
    Resp: Send + Sync + 'static = serde_json::Value,
>: Send + Sync {
    /// Process the request, returning the (possibly modified) request or an error.
    async fn process_request(&self, request: Req) -> Result<Req, Err>;

    /// Process the request and return a [`MiddlewareAction`].
    ///
    /// The default wraps [`process_request`](Self::process_request) in
    /// [`MiddlewareAction::Continue`].
    async fn process_request_action(&self, request: Req) -> Result<MiddlewareAction<Req, Resp>, Err> {
        self.process_request(request).await.map(MiddlewareAction::Continue)
    }
}

/// Middleware that intercepts outbound responses.
#[async_trait]
pub trait ResponseMiddleware<
    Resp: Send + Sync + 'static = serde_json::Value,
    Err: Send + Sync + 'static = IngressError,
>: Send + Sync {
    /// Process the response, returning the (possibly modified) response or an error.
    async fn process_response(&self, response: Resp) -> Result<Resp, Err>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_middleware_is_object_safe() {
        fn _accepts(_m: &dyn RequestMiddleware) {}
    }

    #[test]
    fn test_response_middleware_is_object_safe() {
        fn _accepts(_m: &dyn ResponseMiddleware) {}
    }
}
