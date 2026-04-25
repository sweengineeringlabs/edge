//! Payment method type enum.

use serde::{Deserialize, Serialize};

/// Type of payment method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodType {
    Card,
    BankTransfer,
    Wallet,
    Crypto,
    BuyNowPayLater,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_method_types_are_distinct() {
        assert_ne!(PaymentMethodType::Card, PaymentMethodType::Wallet);
    }
}
