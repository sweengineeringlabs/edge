//! Outbound gateway boundary — wraps egress port adapters.

use std::sync::Arc;

use swe_edge_egress::{DatabaseGateway, GrpcOutbound, HttpOutbound, NotificationSender, PaymentGateway};

/// Holds the egress adapters the daemon uses for outbound calls.
pub struct EgressGateway {
    pub(crate) http:         Arc<dyn HttpOutbound>,
    pub(crate) grpc:         Option<Arc<dyn GrpcOutbound>>,
    pub(crate) database:     Option<Arc<dyn DatabaseGateway>>,
    pub(crate) notification: Option<Arc<dyn NotificationSender>>,
    pub(crate) payment:      Option<Arc<dyn PaymentGateway>>,
}

impl EgressGateway {
    /// Construct a gateway with only an HTTP outbound adapter; all other adapters default to `None`.
    pub fn http(http: Arc<dyn HttpOutbound>) -> Self {
        Self { http, grpc: None, database: None, notification: None, payment: None }
    }

    /// Add (or replace) the gRPC outbound transport.
    pub fn with_grpc(mut self, grpc: Arc<dyn GrpcOutbound>) -> Self {
        self.grpc = Some(grpc);
        self
    }

    /// Add (or replace) the database gateway adapter.
    pub fn with_database(mut self, db: Arc<dyn DatabaseGateway>) -> Self {
        self.database = Some(db);
        self
    }

    /// Add (or replace) the notification sender adapter.
    pub fn with_notification(mut self, n: Arc<dyn NotificationSender>) -> Self {
        self.notification = Some(n);
        self
    }

    /// Add (or replace) the payment gateway adapter.
    pub fn with_payment(mut self, p: Arc<dyn PaymentGateway>) -> Self {
        self.payment = Some(p);
        self
    }
}
