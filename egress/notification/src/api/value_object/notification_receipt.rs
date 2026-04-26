//! Receipt returned after sending a notification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::notification_status::NotificationStatus;

/// Delivery receipt for a sent notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationReceipt {
    pub notification_id: String,
    pub status: NotificationStatus,
    pub provider_id: Option<String>,
    pub sent_at: DateTime<Utc>,
}

impl NotificationReceipt {
    pub fn sent(notification_id: impl Into<String>) -> Self {
        Self {
            notification_id: notification_id.into(),
            status: NotificationStatus::Sent,
            provider_id: None,
            sent_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: sent
    #[test]
    fn test_sent_creates_receipt_with_sent_status() {
        let r = NotificationReceipt::sent("notif-001");
        assert_eq!(r.status, NotificationStatus::Sent);
        assert_eq!(r.notification_id, "notif-001");
    }
}
