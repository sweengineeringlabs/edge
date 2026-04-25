//! Payment egress trait — submits payment requests outbound.

use crate::api::egress_error::EgressError;

/// Submits payment charges and captures to an external payment processor.
pub trait PaymentGateway: Send + Sync {
    /// A description of this payment gateway for diagnostics.
    fn describe(&self) -> &'static str;

    /// Charge an amount (in minor currency units) to the given token.
    fn charge(&self, token: &str, amount_minor: u64, currency: &str) -> Result<String, EgressError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeGateway;
    impl PaymentGateway for FakeGateway {
        fn describe(&self) -> &'static str { "fake" }
        fn charge(&self, _token: &str, _amount: u64, _currency: &str) -> Result<String, EgressError> {
            Ok("txn_001".into())
        }
    }

    #[test]
    fn test_payment_gateway_charge_returns_transaction_id() {
        let id = FakeGateway.charge("tok_123", 100, "USD").unwrap();
        assert!(!id.is_empty());
    }

    #[test]
    fn test_payment_gateway_describe_returns_str() {
        assert_eq!(FakeGateway.describe(), "fake");
    }
}
