//! Console notification sender — logs notifications to stdout (dev/test only).

use futures::future::BoxFuture;

use crate::api::egress_error::EgressResult;
use crate::api::health_check::HealthCheck;
use crate::api::notification::{Notification, NotificationReceipt};
use crate::api::notification_sender::NotificationSender;

/// Sends notifications by printing to stdout.
pub(crate) struct ConsoleNotifier;

impl NotificationSender for ConsoleNotifier {
    fn send(&self, notification: Notification) -> BoxFuture<'_, EgressResult<NotificationReceipt>> {
        Box::pin(async move {
            let subject = notification.subject.as_deref().unwrap_or("(no subject)");
            tracing::info!(
                channel = ?notification.channel,
                recipient = %notification.recipient,
                subject = %subject,
                "notification sent"
            );
            let id = uuid::Uuid::new_v4().to_string();
            Ok(NotificationReceipt::sent(id))
        })
    }

    fn send_batch(&self, notifications: Vec<Notification>) -> BoxFuture<'_, EgressResult<Vec<NotificationReceipt>>> {
        Box::pin(async move {
            let mut receipts = Vec::with_capacity(notifications.len());
            for n in notifications {
                tracing::info!(recipient = %n.recipient, "batch notification sent");
                receipts.push(NotificationReceipt::sent(uuid::Uuid::new_v4().to_string()));
            }
            Ok(receipts)
        })
    }

    fn health_check(&self) -> BoxFuture<'_, EgressResult<HealthCheck>> {
        Box::pin(async move { Ok(HealthCheck::healthy()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::notification::Notification;

    #[tokio::test]
    async fn test_send_returns_receipt_with_sent_status() {
        let n = ConsoleNotifier;
        let notif = Notification::email("dev@test.com", "Hello", "World");
        let r = n.send(notif).await.unwrap();
        assert_eq!(r.status, crate::api::notification::NotificationStatus::Sent);
    }

    #[tokio::test]
    async fn test_send_batch_returns_receipt_per_notification() {
        let n = ConsoleNotifier;
        let batch = vec![
            Notification::email("a@b.com", "S1", "B1"),
            Notification::email("c@d.com", "S2", "B2"),
        ];
        let receipts = n.send_batch(batch).await.unwrap();
        assert_eq!(receipts.len(), 2);
    }

    #[tokio::test]
    async fn test_health_check_returns_healthy() {
        let r = ConsoleNotifier.health_check().await.unwrap();
        assert_eq!(r.status, crate::api::health_check::HealthStatus::Healthy);
    }
}
