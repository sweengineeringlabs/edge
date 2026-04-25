//! Payment transaction.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::money::Money;
use super::payment_status::PaymentStatus;

/// A payment transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub amount: Money,
    pub status: PaymentStatus,
    pub customer_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
}

impl Payment {
    pub fn new(id: impl Into<String>, amount: Money) -> Self {
        Self {
            id: id.into(),
            amount,
            status: PaymentStatus::Pending,
            customer_id: None,
            payment_method_id: None,
            idempotency_key: None,
            created_at: Utc::now(),
            description: None,
        }
    }

    pub fn with_customer(mut self, customer_id: impl Into<String>) -> Self {
        self.customer_id = Some(customer_id.into()); self
    }

    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into()); self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::payment::currency::Currency;

    /// @covers: new
    #[test]
    fn test_new_creates_pending_payment() {
        let p = Payment::new("pay-001", Money::new(500, Currency::usd()));
        assert_eq!(p.id, "pay-001");
        assert_eq!(p.status, PaymentStatus::Pending);
        assert_eq!(p.amount.amount_minor, 500);
    }

    /// @covers: with_customer
    #[test]
    fn test_with_customer_sets_customer_id() {
        let p = Payment::new("pay-002", Money::usd_cents(100)).with_customer("cust-1");
        assert_eq!(p.customer_id, Some("cust-1".to_string()));
    }
}
