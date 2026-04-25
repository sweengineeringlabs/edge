//! Notification egress trait — sends alerts and messages outbound.

use crate::api::error::EgressError;

/// Sends outbound notifications (email, webhook, Slack, etc.).
pub trait NotificationSender: Send + Sync {
    /// A description of this notification sender for diagnostics.
    fn describe(&self) -> &'static str;

    /// Send a notification with a subject and body.
    fn send(&self, subject: &str, body: &str) -> Result<(), EgressError>;
}
