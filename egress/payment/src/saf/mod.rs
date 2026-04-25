//! SAF layer — payment public facade.

pub use crate::api::aggregate::{Customer, Payment};
pub use crate::api::port::{PaymentError, PaymentGateway, PaymentInbound, PaymentOutbound, PaymentPortResult};
pub use crate::api::value_object::{
    CardDetails, Currency, Money, PaymentConfig, PaymentMethod, PaymentMethodType, PaymentProvider,
    PaymentResult, PaymentStatus, Refund, RefundReason, RefundResult, RefundStatus,
};

/// Returns a mock payment gateway (for testing/development).
pub fn mock_payment_gateway_impl() -> impl PaymentGateway {
    crate::core::payment::MockPaymentGateway::new()
}
