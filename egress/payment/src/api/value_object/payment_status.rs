//! Payment transaction status.

use serde::{Deserialize, Serialize};

/// Status of a payment transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Authorized,
    Captured,
    Settled,
    Refunded,
    PartiallyRefunded,
    Cancelled,
    Failed,
    Disputed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_statuses_are_distinct() {
        assert_ne!(PaymentStatus::Captured, PaymentStatus::Failed);
    }
}
