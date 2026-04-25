//! Pipeline and Router traits.

use async_trait::async_trait;
use crate::api::ingress_error::IngressError;

/// Router — dispatches a request to produce a response.
#[async_trait]
pub trait Router<
    Req: Send + Sync + 'static = serde_json::Value,
    Resp: Send + Sync + 'static = serde_json::Value,
    Err: Send + Sync + 'static = IngressError,
>: Send + Sync {
    async fn dispatch(&self, request: &Req) -> Result<Resp, Err>;
}

/// Pipeline — executes a request through an ordered chain of stages.
#[async_trait]
pub trait Pipeline<
    Req: Send + Sync + 'static = serde_json::Value,
    Resp: Send + Sync + 'static = serde_json::Value,
    Err: Send + Sync + 'static = IngressError,
>: Send + Sync {
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
