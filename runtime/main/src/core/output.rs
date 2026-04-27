//! Default [`Output`] implementation.

use std::sync::Arc;

use swe_edge_egress::{DatabaseGateway, GrpcOutbound, HttpOutbound, NotificationSender, PaymentGateway};

use crate::api::output::Output;

/// Holds egress adapters by `Arc`.
pub struct DefaultOutput {
    http:         Arc<dyn HttpOutbound>,
    grpc:         Option<Arc<dyn GrpcOutbound>>,
    database:     Option<Arc<dyn DatabaseGateway>>,
    notification: Option<Arc<dyn NotificationSender>>,
    payment:      Option<Arc<dyn PaymentGateway>>,
}

impl Output for DefaultOutput {
    fn http(&self)         -> Arc<dyn HttpOutbound>              { self.http.clone() }
    fn grpc(&self)         -> Option<Arc<dyn GrpcOutbound>>      { self.grpc.clone() }
    fn database(&self)     -> Option<Arc<dyn DatabaseGateway>>   { self.database.clone() }
    fn notification(&self) -> Option<Arc<dyn NotificationSender>> { self.notification.clone() }
    fn payment(&self)      -> Option<Arc<dyn PaymentGateway>>    { self.payment.clone() }
}

impl DefaultOutput {
    /// Construct a gateway with only an HTTP outbound adapter; all other adapters default to `None`.
    pub fn new_http(http: Arc<dyn HttpOutbound>) -> Self {
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
