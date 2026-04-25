//! Notification priority levels.

use serde::{Deserialize, Serialize};

/// Priority level for a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_priority_default_is_normal() {
        assert_eq!(NotificationPriority::default(), NotificationPriority::Normal);
    }
}
