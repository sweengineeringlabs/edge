//! Payment provider configuration.

use serde::{Deserialize, Serialize};

use super::payment_provider::PaymentProvider;

/// Configuration for a payment provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentConfig {
    pub provider: PaymentProvider,
    pub api_key: Option<String>,
    pub webhook_secret: Option<String>,
    pub sandbox: bool,
    pub timeout_secs: u64,
}

impl PaymentConfig {
    pub fn mock() -> Self {
        Self { provider: PaymentProvider::Mock, api_key: None, webhook_secret: None, sandbox: true, timeout_secs: 5 }
    }

    pub fn stripe(api_key: impl Into<String>) -> Self {
        Self { provider: PaymentProvider::Stripe, api_key: Some(api_key.into()), webhook_secret: None, sandbox: false, timeout_secs: 30 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: mock
    #[test]
    fn test_mock_creates_sandbox_mock_config() {
        let cfg = PaymentConfig::mock();
        assert_eq!(cfg.provider, PaymentProvider::Mock);
        assert!(cfg.sandbox);
        assert!(cfg.api_key.is_none());
    }

    /// @covers: stripe
    #[test]
    fn test_stripe_creates_stripe_config_with_key() {
        let cfg = PaymentConfig::stripe("sk_test_xxx");
        assert_eq!(cfg.provider, PaymentProvider::Stripe);
        assert_eq!(cfg.api_key, Some("sk_test_xxx".to_string()));
    }
}
