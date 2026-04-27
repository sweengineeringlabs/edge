//! Default [`Input`] implementation.

use std::sync::Arc;

use swe_edge_ingress::{FileInbound, GrpcInbound, HttpInbound};

use crate::api::input::Input;

/// Holds optional ingress adapters by `Arc`.
///
/// Every transport is optional at construction time so consumer apps can
/// wire only what they need. At least one must be configured before
/// [`RuntimeManager::start`] is called — the runtime enforces this.
pub struct DefaultInput {
    http: Option<Arc<dyn HttpInbound>>,
    grpc: Option<Arc<dyn GrpcInbound>>,
    file: Option<Arc<dyn FileInbound>>,
}

impl Input for DefaultInput {
    fn http(&self) -> Option<Arc<dyn HttpInbound>> { self.http.clone() }
    fn grpc(&self) -> Option<Arc<dyn GrpcInbound>> { self.grpc.clone() }
    fn file(&self) -> Option<Arc<dyn FileInbound>> { self.file.clone() }
}

impl DefaultInput {
    /// Start with HTTP as the sole transport.
    pub fn new_http(http: Arc<dyn HttpInbound>) -> Self {
        Self { http: Some(http), grpc: None, file: None }
    }

    /// Start with gRPC as the sole transport.
    pub fn new_grpc(grpc: Arc<dyn GrpcInbound>) -> Self {
        Self { http: None, grpc: Some(grpc), file: None }
    }

    /// Start with file as the sole transport.
    pub fn new_file(file: Arc<dyn FileInbound>) -> Self {
        Self { http: None, grpc: None, file: Some(file) }
    }

    /// Construct with no transports configured (used to test the no-ingress guard).
    pub fn empty() -> Self {
        Self { http: None, grpc: None, file: None }
    }

    /// Add (or replace) the HTTP transport.
    pub fn with_http(mut self, http: Arc<dyn HttpInbound>) -> Self {
        self.http = Some(http); self
    }

    /// Add (or replace) the gRPC transport.
    pub fn with_grpc(mut self, grpc: Arc<dyn GrpcInbound>) -> Self {
        self.grpc = Some(grpc); self
    }

    /// Add (or replace) the file transport.
    pub fn with_file(mut self, file: Arc<dyn FileInbound>) -> Self {
        self.file = Some(file); self
    }
}
