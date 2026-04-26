//! Inbound gateway boundary — wraps ingress port adapters.

use std::sync::Arc;

use swe_edge_ingress::{HttpInbound, GrpcInbound, FileInbound};

/// Holds the bound ingress adapters the daemon serves traffic through.
pub struct IngressGateway {
    pub(crate) http:  Arc<dyn HttpInbound>,
    pub(crate) grpc:  Option<Arc<dyn GrpcInbound>>,
    pub(crate) file:  Option<Arc<dyn FileInbound>>,
}

impl IngressGateway {
    pub fn http(http: Arc<dyn HttpInbound>) -> Self {
        Self { http, grpc: None, file: None }
    }

    pub fn with_grpc(mut self, grpc: Arc<dyn GrpcInbound>) -> Self {
        self.grpc = Some(grpc);
        self
    }

    pub fn with_file(mut self, file: Arc<dyn FileInbound>) -> Self {
        self.file = Some(file);
        self
    }
}
