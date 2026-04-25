//! Refund types.

use serde::{Deserialize, Serialize};

use super::money::Money;
use super::payment_status::PaymentStatus;

/// Reason for a refund.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefundReason {
    RequestedByCustomer,
    Duplicate,
    Fraudulent,
    ProductNotReceived,
    ProductUnacceptable,
    Other,
}

/// Status of a refund.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefundStatus {
    Pending,
    Succeeded,
    Failed,
    Cancelled,
}

/// A refund request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refund {
    pub payment_id: String,
    pub amount: Option<Money>,
    pub reason: RefundReason,
    pub idempotency_key: Option<String>,
}

impl Refund {
    pub fn full(payment_id: impl Into<String>, reason: RefundReason) -> Self {
        Self { payment_id: payment_id.into(), amount: None, reason, idempotency_key: None }
    }

    pub fn partial(payment_id: impl Into<String>, amount: Money, reason: RefundReason) -> Self {
        Self { payment_id: payment_id.into(), amount: Some(amount), reason, idempotency_key: None }
    }
}

/// Result of a refund operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResult {
    pub refund_id: String,
    pub payment_id: String,
    pub status: RefundStatus,
    pub payment_status: PaymentStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: full
    #[test]
    fn test_full_creates_refund_without_amount() {
        let r = Refund::full("pay-1", RefundReason::RequestedByCustomer);
        assert!(r.amount.is_none());
        assert_eq!(r.reason, RefundReason::RequestedByCustomer);
    }

    /// @covers: partial
    #[test]
    fn test_partial_creates_refund_with_amount() {
        let r = Refund::partial("pay-1", Money::usd_cents(500), RefundReason::Duplicate);
        assert!(r.amount.is_some());
    }
}
