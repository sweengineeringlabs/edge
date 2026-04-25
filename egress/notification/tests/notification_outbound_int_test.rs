//! Integration tests for the notification outbound domain.

use swe_edge_egress_notification::{
    console_notifier_impl, Notification, NotificationSender, NotificationStatus,
};

/// @covers: console_notifier_impl — send returns sent receipt.
#[tokio::test]
async fn test_console_notifier_send_returns_sent_receipt() {
    let n = console_notifier_impl();
    let notif = Notification::email("user@example.com", "Welcome", "Hello there");
    let receipt = n.send(notif).await.unwrap();
    assert_eq!(receipt.status, NotificationStatus::Sent);
    assert!(!receipt.notification_id.is_empty(), "receipt must have a non-empty id");
}

/// @covers: console_notifier_impl — batch send returns one receipt per notification.
#[tokio::test]
async fn test_console_notifier_send_batch_returns_receipt_per_item() {
    let n = console_notifier_impl();
    let batch = vec![
        Notification::email("a@b.com", "Sub1", "Body1"),
        Notification::sms("+155500001", "Alert"),
    ];
    let receipts = n.send_batch(batch).await.unwrap();
    assert_eq!(receipts.len(), 2, "expected one receipt per notification");
    for r in &receipts {
        assert_eq!(r.status, NotificationStatus::Sent);
    }
}
