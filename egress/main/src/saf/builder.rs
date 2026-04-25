//! SAF builder — constructs outbound adapters.

use crate::api::outbound_sink::OutputSink;
use crate::api::traits::Validator;
use crate::core::output::StdoutSink;
use crate::core::validator::PassthroughValidator;
use swe_edge_egress_database::DatabaseGateway;
use swe_edge_egress_notification::NotificationSender;
use swe_edge_egress_payment::PaymentGateway;

/// Returns an in-memory database adapter (for testing/development).
pub fn memory_database() -> impl DatabaseGateway {
    swe_edge_egress_database::memory_database_impl()
}

/// Returns a console notification sender (for testing/development).
pub fn console_notifier() -> impl NotificationSender {
    swe_edge_egress_notification::console_notifier_impl()
}

/// Returns a mock payment gateway (for testing/development).
pub fn mock_payment_gateway() -> impl PaymentGateway {
    swe_edge_egress_payment::mock_payment_gateway_impl()
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
    use swe_edge_egress_database::{DatabaseRead, DatabaseWrite, Record};
    use swe_edge_egress_notification::Notification;
    use swe_edge_egress_payment::{Money, Payment, PaymentOutbound, PaymentStatus};
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
        assert_eq!(r.status, swe_edge_egress_notification::NotificationStatus::Sent);
    }

    /// @covers: mock_payment_gateway
    #[tokio::test]
    async fn test_mock_payment_gateway_charge_returns_captured() {
        let gw = mock_payment_gateway();
        let r = gw.charge(Payment::new("pay-1", Money::usd_cents(500))).await.unwrap();
        assert_eq!(r.status, PaymentStatus::Captured);
    }

    /// @covers: stdout_sink
    #[tokio::test]
    async fn test_stdout_sink_write_succeeds() {
        use crate::api::outbound_sink::OutputSink;
        let s = stdout_sink();
        assert!(s.write(b"ok\n".to_vec()).await.is_ok());
    }
}
