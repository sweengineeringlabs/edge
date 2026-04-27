//! Outbound gateway contract.

use std::sync::Arc;

use swe_edge_egress::{DatabaseGateway, GrpcOutbound, HttpOutbound, NotificationSender, PaymentGateway};

/// Supplies the egress adapters the runtime uses for outbound calls.
pub trait Output: Send + Sync {
    /// HTTP outbound adapter (required).
    fn http(&self) -> Arc<dyn HttpOutbound>;
    /// gRPC outbound adapter, if configured.
    fn grpc(&self) -> Option<Arc<dyn GrpcOutbound>>;
    /// Database gateway adapter, if configured.
    fn database(&self) -> Option<Arc<dyn DatabaseGateway>>;
    /// Notification sender adapter, if configured.
    fn notification(&self) -> Option<Arc<dyn NotificationSender>>;
    /// Payment gateway adapter, if configured.
    fn payment(&self) -> Option<Arc<dyn PaymentGateway>>;
}
