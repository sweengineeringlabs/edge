//! PaymentGateway trait — submits payment requests to an external processor.

use futures::future::BoxFuture;
use thiserror::Error;

use crate::api::aggregate::{Customer, Payment};
use crate::api::value_object::{PaymentMethod, PaymentResult, Refund, RefundResult};

/// Error type for payment operations.
#[derive(Debug, Error)]
pub enum PaymentError {
    #[error("payment declined: {0}")]
    Declined(String),
    #[error("invalid payment: {0}")]
    InvalidPayment(String),
    #[error("customer not found: {0}")]
    CustomerNotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("provider error: {0}")]
    ProviderError(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Result type for payment operations.
pub type PaymentPortResult<T> = Result<T, PaymentError>;

/// Inbound payment operations (read/query).
pub trait PaymentInbound: Send + Sync {
    fn get_payment(&self, payment_id: &str) -> BoxFuture<'_, PaymentPortResult<Option<Payment>>>;
    fn get_customer(&self, customer_id: &str) -> BoxFuture<'_, PaymentPortResult<Option<Customer>>>;
    fn list_payment_methods(&self, customer_id: &str) -> BoxFuture<'_, PaymentPortResult<Vec<PaymentMethod>>>;
    fn health_check(&self) -> BoxFuture<'_, PaymentPortResult<()>>;
}

/// Outbound payment operations (charge/refund/customer management).
pub trait PaymentOutbound: Send + Sync {
    fn charge(&self, payment: Payment) -> BoxFuture<'_, PaymentPortResult<PaymentResult>>;
    fn refund(&self, refund: Refund) -> BoxFuture<'_, PaymentPortResult<RefundResult>>;
    fn create_customer(&self, customer: Customer) -> BoxFuture<'_, PaymentPortResult<Customer>>;
    fn attach_payment_method(&self, customer_id: &str, method: PaymentMethod) -> BoxFuture<'_, PaymentPortResult<PaymentMethod>>;
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
