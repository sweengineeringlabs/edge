//! Middleware traits for request/response pipeline processing.

use async_trait::async_trait;
use crate::api::ingress_error::IngressError;

/// Action returned by request middleware.
pub enum MiddlewareAction<Req, Resp> {
    Continue(Req),
    ShortCircuit(Resp),
}

/// Middleware that intercepts inbound requests.
#[async_trait]
pub trait RequestMiddleware<
    Req: Send + Sync + 'static = serde_json::Value,
    Err: Send + Sync + 'static = IngressError,
    Resp: Send + Sync + 'static = serde_json::Value,
>: Send + Sync {
    async fn process_request(&self, request: Req) -> Result<Req, Err>;

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
