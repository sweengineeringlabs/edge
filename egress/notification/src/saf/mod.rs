//! SAF layer — notification public facade.

pub use crate::api::aggregate::Notification;
pub use crate::api::port::{NotificationError, NotificationResult, NotificationSender};
pub use crate::api::value_object::{
    EmailConfig, NotificationChannel, NotificationConfig, NotificationPriority,
    NotificationReceipt, NotificationStatus, PushConfig, SmsConfig, WebhookConfig,
};

/// Returns a console notification sender (for testing/development).
pub fn console_notifier_impl() -> impl NotificationSender {
    crate::core::notification::ConsoleNotifier
}
