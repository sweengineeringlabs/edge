//! HTTP outbound trait — makes outbound HTTP requests.

use futures::future::BoxFuture;

use crate::api::egress_error::EgressResult;
use crate::api::health_check::HealthCheck;
use crate::api::http::{HttpRequest, HttpResponse};

/// Makes outbound HTTP requests to external services.
pub trait HttpOutbound: Send + Sync {
    fn send(&self, request: HttpRequest) -> BoxFuture<'_, EgressResult<HttpResponse>>;
    fn health_check(&self) -> BoxFuture<'_, EgressResult<HealthCheck>>;

    fn get(&self, url: &str) -> BoxFuture<'_, EgressResult<HttpResponse>> {
        let req = HttpRequest::get(url.to_string());
        self.send(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_outbound_is_object_safe() {
        fn _assert_object_safe(_: &dyn HttpOutbound) {}
    }
}
