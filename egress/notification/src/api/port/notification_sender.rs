//! NotificationSender trait — sends outbound notifications.

use futures::future::BoxFuture;
use thiserror::Error;

use crate::api::aggregate::Notification;
use crate::api::value_object::NotificationReceipt;

/// Error type for notification operations.
#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("delivery failed: {0}")]
    DeliveryFailed(String),
    #[error("invalid recipient: {0}")]
    InvalidRecipient(String),
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Result type for notification operations.
pub type NotificationResult<T> = Result<T, NotificationError>;

/// Sends outbound notifications via email, SMS, push, or webhook.
pub trait NotificationSender: Send + Sync {
    fn send(&self, notification: Notification) -> BoxFuture<'_, NotificationResult<NotificationReceipt>>;
    fn send_batch(&self, notifications: Vec<Notification>) -> BoxFuture<'_, NotificationResult<Vec<NotificationReceipt>>>;
    fn health_check(&self) -> BoxFuture<'_, NotificationResult<()>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_sender_is_object_safe() {
        fn _assert_object_safe(_: &dyn NotificationSender) {}
    }
}
