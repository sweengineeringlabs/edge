//! Result of a payment charge operation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::payment_status::PaymentStatus;

/// Result returned after a payment charge attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResult {
    pub payment_id: String,
    pub provider_transaction_id: Option<String>,
    pub status: PaymentStatus,
    pub processed_at: DateTime<Utc>,
    pub failure_reason: Option<String>,
}

impl PaymentResult {
    pub fn captured(payment_id: impl Into<String>, provider_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            provider_transaction_id: Some(provider_id.into()),
            status: PaymentStatus::Captured,
            processed_at: Utc::now(),
            failure_reason: None,
        }
    }

    pub fn failed(payment_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            provider_transaction_id: None,
            status: PaymentStatus::Failed,
            processed_at: Utc::now(),
            failure_reason: Some(reason.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: captured
    #[test]
    fn test_captured_creates_successful_payment_result() {
        let r = PaymentResult::captured("pay-1", "ch_abc");
        assert_eq!(r.status, PaymentStatus::Captured);
        assert!(r.failure_reason.is_none());
    }

    /// @covers: failed
    #[test]
    fn test_failed_creates_failed_payment_result_with_reason() {
        let r = PaymentResult::failed("pay-2", "card declined");
        assert_eq!(r.status, PaymentStatus::Failed);
        assert_eq!(r.failure_reason, Some("card declined".to_string()));
    }
}
