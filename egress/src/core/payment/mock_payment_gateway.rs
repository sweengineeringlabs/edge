//! Mock payment gateway — for testing and development.

use std::collections::HashMap;

use futures::future::BoxFuture;
use parking_lot::RwLock;

use crate::api::egress_error::{EgressError, EgressResult};
use crate::api::health_check::HealthCheck;
use crate::api::payment::{
    Customer, Payment, PaymentMethod, PaymentResult, Refund, RefundResult, RefundStatus,
};
use crate::api::payment_gateway::{PaymentGateway, PaymentInbound, PaymentOutbound};

/// In-memory mock payment gateway for tests.
pub(crate) struct MockPaymentGateway {
    payments: RwLock<HashMap<String, Payment>>,
    customers: RwLock<HashMap<String, Customer>>,
}

impl MockPaymentGateway {
    pub(crate) fn new() -> Self {
        Self { payments: RwLock::new(HashMap::new()), customers: RwLock::new(HashMap::new()) }
    }
}

impl PaymentInbound for MockPaymentGateway {
    fn get_payment(&self, payment_id: &str) -> BoxFuture<'_, EgressResult<Option<Payment>>> {
        let id = payment_id.to_string();
        Box::pin(async move {
            Ok(self.payments.read().get(&id).cloned())
        })
    }

    fn get_customer(&self, customer_id: &str) -> BoxFuture<'_, EgressResult<Option<Customer>>> {
        let id = customer_id.to_string();
        Box::pin(async move {
            Ok(self.customers.read().get(&id).cloned())
        })
    }

    fn list_payment_methods(&self, _customer_id: &str) -> BoxFuture<'_, EgressResult<Vec<PaymentMethod>>> {
        Box::pin(async move { Ok(vec![]) })
    }

    fn health_check(&self) -> BoxFuture<'_, EgressResult<HealthCheck>> {
        Box::pin(async move { Ok(HealthCheck::healthy()) })
    }
}

impl PaymentOutbound for MockPaymentGateway {
    fn charge(&self, payment: Payment) -> BoxFuture<'_, EgressResult<PaymentResult>> {
        Box::pin(async move {
            let provider_id = format!("mock_txn_{}", uuid::Uuid::new_v4());
            let result = PaymentResult::captured(payment.id.clone(), &provider_id);
            self.payments.write().insert(payment.id.clone(), payment);
            Ok(result)
        })
    }

    fn refund(&self, refund: Refund) -> BoxFuture<'_, EgressResult<RefundResult>> {
        Box::pin(async move {
            Ok(RefundResult {
                refund_id: uuid::Uuid::new_v4().to_string(),
                payment_id: refund.payment_id.clone(),
                status: RefundStatus::Succeeded,
                payment_status: crate::api::payment::PaymentStatus::Refunded,
            })
        })
    }

    fn create_customer(&self, customer: Customer) -> BoxFuture<'_, EgressResult<Customer>> {
        Box::pin(async move {
            if self.customers.read().contains_key(&customer.id) {
                return Err(EgressError::AlreadyExists(format!("customer {}", customer.id)));
            }
            self.customers.write().insert(customer.id.clone(), customer.clone());
            Ok(customer)
        })
    }

    fn attach_payment_method(&self, _customer_id: &str, method: PaymentMethod) -> BoxFuture<'_, EgressResult<PaymentMethod>> {
        Box::pin(async move { Ok(method) })
    }
}

impl PaymentGateway for MockPaymentGateway {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::payment::{Currency, Money, Payment};

    fn gw() -> MockPaymentGateway { MockPaymentGateway::new() }

    #[tokio::test]
    async fn test_charge_returns_captured_result() {
        let g = gw();
        let p = Payment::new("pay-1", Money::usd_cents(1000));
        let r = g.charge(p).await.unwrap();
        assert_eq!(r.status, crate::api::payment::PaymentStatus::Captured);
        assert!(r.provider_transaction_id.is_some());
    }

    #[tokio::test]
    async fn test_get_payment_after_charge_returns_payment() {
        let g = gw();
        let p = Payment::new("pay-2", Money::usd_cents(500));
        g.charge(p).await.unwrap();
        let found = g.get_payment("pay-2").await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_create_customer_stores_customer() {
        let g = gw();
        let c = Customer::new("cust-1").with_email("a@b.com");
        g.create_customer(c).await.unwrap();
        let found = g.get_customer("cust-1").await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_create_duplicate_customer_returns_already_exists_error() {
        let g = gw();
        g.create_customer(Customer::new("c1")).await.unwrap();
        let r = g.create_customer(Customer::new("c1")).await;
        assert!(matches!(r, Err(EgressError::AlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_refund_returns_succeeded_status() {
        let g = gw();
        let r = g.refund(crate::api::payment::Refund::full("pay-1", crate::api::payment::RefundReason::RequestedByCustomer)).await.unwrap();
        assert_eq!(r.status, RefundStatus::Succeeded);
    }

    #[tokio::test]
    async fn test_health_check_returns_healthy() {
        let r = gw().health_check().await.unwrap();
        assert_eq!(r.status, crate::api::health_check::HealthStatus::Healthy);
    }
}
