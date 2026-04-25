//! HTTP inbound trait — handles incoming HTTP requests.

use futures::future::BoxFuture;

use crate::api::health_check::HealthCheck;
use crate::api::http::{HttpRequest, HttpResponse};
use crate::api::ingress_error::IngressResult;

/// Receives and handles inbound HTTP requests.
pub trait HttpInbound: Send + Sync {
    fn handle(&self, request: HttpRequest) -> BoxFuture<'_, IngressResult<HttpResponse>>;
    fn health_check(&self) -> BoxFuture<'_, IngressResult<HealthCheck>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_inbound_is_object_safe() {
        fn _assert_object_safe(_: &dyn HttpInbound) {}
    }
}
