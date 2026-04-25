//! Payment provider enum.

use serde::{Deserialize, Serialize};

/// Supported payment providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentProvider {
    Stripe,
    Braintree,
    Adyen,
    PayPal,
    Mock,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_provider_variants_are_distinct() {
        assert_ne!(PaymentProvider::Stripe, PaymentProvider::Mock);
    }
}
