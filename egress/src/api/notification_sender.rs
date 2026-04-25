//! Notification egress trait — sends alerts and messages outbound.

use crate::api::egress_error::EgressError;

/// Sends outbound notifications (email, webhook, Slack, etc.).
pub trait NotificationSender: Send + Sync {
    /// A description of this notification sender for diagnostics.
    fn describe(&self) -> &'static str;

    /// Send a notification with a subject and body.
    fn send(&self, subject: &str, body: &str) -> Result<(), EgressError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopSender;
    impl NotificationSender for NoopSender {
        fn describe(&self) -> &'static str { "noop" }
        fn send(&self, _subject: &str, _body: &str) -> Result<(), EgressError> { Ok(()) }
    }

    #[test]
    fn test_notification_sender_send_succeeds() {
        assert!(NoopSender.send("hi", "world").is_ok());
    }

    #[test]
    fn test_notification_sender_describe_returns_str() {
        assert_eq!(NoopSender.describe(), "noop");
    }
}
