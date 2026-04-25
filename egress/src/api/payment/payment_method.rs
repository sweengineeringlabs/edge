//! Payment method.

use serde::{Deserialize, Serialize};

use super::card_details::CardDetails;
use super::payment_method_type::PaymentMethodType;

/// A customer's payment method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub id: String,
    pub method_type: PaymentMethodType,
    pub card: Option<CardDetails>,
}

impl PaymentMethod {
    pub fn card(id: impl Into<String>, details: CardDetails) -> Self {
        Self { id: id.into(), method_type: PaymentMethodType::Card, card: Some(details) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: card
    #[test]
    fn test_card_creates_card_payment_method() {
        let pm = PaymentMethod::card("pm_001", CardDetails::from_token("tok_abc"));
        assert_eq!(pm.method_type, PaymentMethodType::Card);
        assert!(pm.card.is_some());
    }
}
