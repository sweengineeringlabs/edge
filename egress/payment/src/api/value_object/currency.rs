//! Currency codes (ISO 4217).

use serde::{Deserialize, Serialize};

/// An ISO 4217 currency code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Currency(pub String);

impl Currency {
    pub fn usd() -> Self { Self("USD".into()) }
    pub fn eur() -> Self { Self("EUR".into()) }
    pub fn gbp() -> Self { Self("GBP".into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl From<&str> for Currency {
    fn from(s: &str) -> Self { Self(s.to_uppercase()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_usd_returns_usd_code() {
        assert_eq!(Currency::usd().as_str(), "USD");
    }

    #[test]
    fn test_currency_from_str_uppercases() {
        let c = Currency::from("usd");
        assert_eq!(c.as_str(), "USD");
    }
}
