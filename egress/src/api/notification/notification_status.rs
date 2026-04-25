//! Notification delivery status.

use serde::{Deserialize, Serialize};

/// Delivery status of a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationStatus {
    Queued,
    Sent,
    Delivered,
    Failed,
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_statuses_are_distinct() {
        assert_ne!(NotificationStatus::Sent, NotificationStatus::Failed);
    }
}
