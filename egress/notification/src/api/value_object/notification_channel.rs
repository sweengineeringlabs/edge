//! Notification delivery channel.

use serde::{Deserialize, Serialize};

/// The channel through which a notification is delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationChannel {
    Email,
    Sms,
    Push,
    Webhook,
    Console,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_channels_are_distinct() {
        assert_ne!(NotificationChannel::Email, NotificationChannel::Sms);
    }
}
