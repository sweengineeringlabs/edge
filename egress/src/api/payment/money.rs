//! Monetary amount in minor currency units.

use serde::{Deserialize, Serialize};

use super::currency::Currency;

/// A monetary amount in minor currency units (e.g., cents).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor: i64,
    pub currency: Currency,
}

impl Money {
    pub fn new(amount_minor: i64, currency: Currency) -> Self {
        Self { amount_minor, currency }
    }

    pub fn usd_cents(cents: i64) -> Self {
        Self::new(cents, Currency::usd())
    }

    pub fn is_positive(&self) -> bool { self.amount_minor > 0 }

    pub fn is_zero(&self) -> bool { self.amount_minor == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: usd_cents
    #[test]
    fn test_usd_cents_creates_money_in_usd() {
        let m = Money::usd_cents(1000);
        assert_eq!(m.amount_minor, 1000);
        assert_eq!(m.currency.as_str(), "USD");
        assert!(m.is_positive());
    }

    /// @covers: is_zero
    #[test]
    fn test_is_zero_returns_true_for_zero_amount() {
        let m = Money::usd_cents(0);
        assert!(m.is_zero());
        assert!(!m.is_positive());
    }
}
