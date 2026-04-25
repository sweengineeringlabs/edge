//! NotificationSender trait — sends outbound notifications.

use futures::future::BoxFuture;

use crate::api::egress_error::EgressResult;
use crate::api::health_check::HealthCheck;
use crate::api::notification::{Notification, NotificationReceipt};

/// Sends outbound notifications via email, SMS, push, or webhook.
pub trait NotificationSender: Send + Sync {
    fn send(&self, notification: Notification) -> BoxFuture<'_, EgressResult<NotificationReceipt>>;
    fn send_batch(&self, notifications: Vec<Notification>) -> BoxFuture<'_, EgressResult<Vec<NotificationReceipt>>>;
    fn health_check(&self) -> BoxFuture<'_, EgressResult<HealthCheck>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_sender_is_object_safe() {
        fn _assert_object_safe(_: &dyn NotificationSender) {}
    }
}
