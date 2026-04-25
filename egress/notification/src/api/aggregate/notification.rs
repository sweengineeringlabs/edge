//! Notification message type.

use serde::{Deserialize, Serialize};

use crate::api::value_object::notification_channel::NotificationChannel;
use crate::api::value_object::notification_priority::NotificationPriority;

/// An outbound notification message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Option<String>,
    pub channel: NotificationChannel,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub priority: NotificationPriority,
    pub idempotency_key: Option<String>,
}

impl Notification {
    pub fn email(recipient: impl Into<String>, subject: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: None,
            channel: NotificationChannel::Email,
            recipient: recipient.into(),
            subject: Some(subject.into()),
            body: body.into(),
            priority: NotificationPriority::Normal,
            idempotency_key: None,
        }
    }

    pub fn sms(recipient: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: None,
            channel: NotificationChannel::Sms,
            recipient: recipient.into(),
            subject: None,
            body: body.into(),
            priority: NotificationPriority::Normal,
            idempotency_key: None,
        }
    }

    pub fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority; self
    }

    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into()); self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: email
    #[test]
    fn test_email_creates_email_notification_with_subject_and_body() {
        let n = Notification::email("a@b.com", "Hello", "World");
        assert_eq!(n.channel, NotificationChannel::Email);
        assert_eq!(n.recipient, "a@b.com");
        assert_eq!(n.subject, Some("Hello".to_string()));
    }

    /// @covers: sms
    #[test]
    fn test_sms_creates_sms_notification_without_subject() {
        let n = Notification::sms("+1234", "msg");
        assert_eq!(n.channel, NotificationChannel::Sms);
        assert!(n.subject.is_none());
    }

    /// @covers: with_priority
    #[test]
    fn test_with_priority_sets_priority() {
        let n = Notification::email("x", "s", "b").with_priority(NotificationPriority::Critical);
        assert_eq!(n.priority, NotificationPriority::Critical);
    }
}
