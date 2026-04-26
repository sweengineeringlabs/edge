//! Outbound gateway boundary — wraps egress port adapters.

use std::sync::Arc;

use swe_edge_egress::{DatabaseGateway, HttpOutbound, NotificationSender, PaymentGateway};

/// Holds the egress adapters the daemon uses for outbound calls.
pub struct EgressGateway {
    pub(crate) http:         Arc<dyn HttpOutbound>,
    pub(crate) database:     Option<Arc<dyn DatabaseGateway>>,
    pub(crate) notification: Option<Arc<dyn NotificationSender>>,
    pub(crate) payment:      Option<Arc<dyn PaymentGateway>>,
}

impl EgressGateway {
    pub fn http(http: Arc<dyn HttpOutbound>) -> Self {
        Self { http, database: None, notification: None, payment: None }
    }

    pub fn with_database(mut self, db: Arc<dyn DatabaseGateway>) -> Self {
        self.database = Some(db);
        self
    }

    pub fn with_notification(mut self, n: Arc<dyn NotificationSender>) -> Self {
        self.notification = Some(n);
        self
    }

    pub fn with_payment(mut self, p: Arc<dyn PaymentGateway>) -> Self {
        self.payment = Some(p);
        self
    }
}
