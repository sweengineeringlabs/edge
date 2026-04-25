//! Notification backend configuration.

use serde::{Deserialize, Serialize};

/// Email provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub from_address: String,
    pub use_tls: bool,
}

/// SMS provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    pub api_url: String,
    pub api_key: String,
    pub from_number: String,
}

/// Push notification configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushConfig {
    pub fcm_server_key: Option<String>,
    pub apns_key_id: Option<String>,
    pub apns_team_id: Option<String>,
}

/// Webhook notification configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub secret: Option<String>,
    pub timeout_secs: u64,
}

impl WebhookConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into(), secret: None, timeout_secs: 30 }
    }
}

/// Umbrella configuration for a notification provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationConfig {
    Email(EmailConfig),
    Sms(SmsConfig),
    Push(PushConfig),
    Webhook(WebhookConfig),
    Console,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_config_new_sets_url_and_defaults() {
        let cfg = WebhookConfig::new("https://hooks.example.com/in");
        assert_eq!(cfg.url, "https://hooks.example.com/in");
        assert_eq!(cfg.timeout_secs, 30);
        assert!(cfg.secret.is_none());
    }

    #[test]
    fn test_notification_config_console_variant_exists() {
        let cfg = NotificationConfig::Console;
        assert!(matches!(cfg, NotificationConfig::Console));
    }
}
