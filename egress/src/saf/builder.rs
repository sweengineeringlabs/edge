//! SAF builder — constructs outbound adapters.

use crate::api::database::DatabaseGateway;
use crate::api::notification_sender::NotificationSender;
use crate::api::outbound_sink::OutputSink;
use crate::api::payment_gateway::PaymentGateway;
use crate::api::traits::Validator;
use crate::core::database::MemoryDatabase;
use crate::core::notification::ConsoleNotifier;
use crate::core::output::StdoutSink;
use crate::core::payment::MockPaymentGateway;
use crate::core::validator::PassthroughValidator;

/// Returns an in-memory database adapter (for testing/development).
pub fn memory_database() -> impl DatabaseGateway {
    MemoryDatabase::new()
}

/// Returns a console notification sender (for testing/development).
pub fn console_notifier() -> impl NotificationSender {
    ConsoleNotifier
}

/// Returns a mock payment gateway (for testing/development).
pub fn mock_payment_gateway() -> impl PaymentGateway {
    MockPaymentGateway::new()
}

/// Returns a stdout output sink (for testing/development).
pub fn stdout_sink() -> impl OutputSink {
    StdoutSink
}

/// Returns the default passthrough validator (accepts all output).
pub fn passthrough_validator() -> impl Validator {
    PassthroughValidator
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::database::{DatabaseRead, DatabaseWrite, Record};
    use crate::api::notification::Notification;
    use crate::api::payment::{Customer, Money, Payment};
    use crate::api::payment_gateway::{PaymentInbound, PaymentOutbound};
    use serde_json::Value;

    /// @covers: memory_database
    #[tokio::test]
    async fn test_memory_database_returns_database_gateway() {
        let db = memory_database();
        let rec = Record::new().set("id", Value::String("k".into()));
        db.insert("t", rec).await.unwrap();
        let found = db.get_by_id("t", "k").await.unwrap();
        assert!(found.is_some());
    }

    /// @covers: passthrough_validator
    #[test]
    fn test_passthrough_validator_accepts_all_output() {
        let v = passthrough_validator();
        assert!(v.is_valid("payload"));
        assert!(v.is_valid(""));
    }

    /// @covers: console_notifier
    #[tokio::test]
    async fn test_console_notifier_send_returns_receipt() {
        let n = console_notifier();
        let r = n.send(Notification::email("x@y.com", "Hi", "Body")).await.unwrap();
        assert_eq!(r.status, crate::api::notification::NotificationStatus::Sent);
    }

    /// @covers: mock_payment_gateway
    #[tokio::test]
    async fn test_mock_payment_gateway_charge_returns_captured() {
        let gw = mock_payment_gateway();
        let r = gw.charge(Payment::new("pay-1", Money::usd_cents(500))).await.unwrap();
        assert_eq!(r.status, crate::api::payment::PaymentStatus::Captured);
    }

    /// @covers: stdout_sink
    #[tokio::test]
    async fn test_stdout_sink_write_succeeds() {
        let s = stdout_sink();
        assert!(s.write(b"ok\n".to_vec()).await.is_ok());
    }
}
