//! Payment egress trait — submits payment requests outbound.

use crate::api::error::EgressError;

/// Submits payment charges and captures to an external payment processor.
pub trait PaymentGateway: Send + Sync {
    /// A description of this payment gateway for diagnostics.
    fn describe(&self) -> &'static str;

    /// Charge an amount (in minor currency units) to the given token.
    fn charge(&self, token: &str, amount_minor: u64, currency: &str) -> Result<String, EgressError>;
}
