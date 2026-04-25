//! PaymentGateway trait — submits payment requests to an external processor.

use futures::future::BoxFuture;

use crate::api::egress_error::EgressResult;
use crate::api::health_check::HealthCheck;
use crate::api::payment::{
    Customer, Payment, PaymentMethod, PaymentResult, Refund, RefundResult,
};

/// Inbound payment operations (read/query).
pub trait PaymentInbound: Send + Sync {
    fn get_payment(&self, payment_id: &str) -> BoxFuture<'_, EgressResult<Option<Payment>>>;
    fn get_customer(&self, customer_id: &str) -> BoxFuture<'_, EgressResult<Option<Customer>>>;
    fn list_payment_methods(&self, customer_id: &str) -> BoxFuture<'_, EgressResult<Vec<PaymentMethod>>>;
    fn health_check(&self) -> BoxFuture<'_, EgressResult<HealthCheck>>;
}

/// Outbound payment operations (charge/refund/customer management).
pub trait PaymentOutbound: Send + Sync {
    fn charge(&self, payment: Payment) -> BoxFuture<'_, EgressResult<PaymentResult>>;
    fn refund(&self, refund: Refund) -> BoxFuture<'_, EgressResult<RefundResult>>;
    fn create_customer(&self, customer: Customer) -> BoxFuture<'_, EgressResult<Customer>>;
    fn attach_payment_method(&self, customer_id: &str, method: PaymentMethod) -> BoxFuture<'_, EgressResult<PaymentMethod>>;
}

/// Full payment gateway — composes inbound and outbound payment operations.
pub trait PaymentGateway: PaymentInbound + PaymentOutbound {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_gateway_is_object_safe() {
        fn _assert_object_safe(_: &dyn PaymentGateway) {}
    }
}
