//! Integration tests for the payment outbound domain.

use swe_edge_egress_payment::{
    mock_payment_gateway_impl, Customer, Money, Payment, PaymentInbound,
    PaymentOutbound, PaymentStatus, Refund, RefundReason, RefundStatus,
};

/// @covers: mock_payment_gateway_impl — charge returns captured status.
#[tokio::test]
async fn test_mock_payment_gateway_charge_returns_captured() {
    let gw = mock_payment_gateway_impl();
    let payment = Payment::new("pay-test-1", Money::usd_cents(1500));
    let result = gw.charge(payment).await.unwrap();
    assert_eq!(result.status, PaymentStatus::Captured);
    assert!(
        result.provider_transaction_id.is_some(),
        "provider transaction id must be present after charge"
    );
}

/// @covers: PaymentInbound::get_payment — after charge, payment is retrievable.
#[tokio::test]
async fn test_mock_payment_gateway_get_payment_after_charge() {
    let gw = mock_payment_gateway_impl();
    let payment = Payment::new("pay-test-2", Money::usd_cents(500));
    gw.charge(payment).await.unwrap();
    let found = gw.get_payment("pay-test-2").await.unwrap();
    assert!(found.is_some(), "payment must be retrievable after charge");
}

/// @covers: PaymentOutbound::create_customer — stores and retrieves customer.
#[tokio::test]
async fn test_mock_payment_gateway_create_and_get_customer() {
    let gw = mock_payment_gateway_impl();
    let customer = Customer::new("cust-int-1").with_email("test@example.com");
    gw.create_customer(customer).await.unwrap();
    let found = gw.get_customer("cust-int-1").await.unwrap();
    assert!(found.is_some(), "customer must be found after creation");
    assert_eq!(found.unwrap().email, Some("test@example.com".to_string()));
}

/// @covers: PaymentOutbound::refund — returns succeeded status.
#[tokio::test]
async fn test_mock_payment_gateway_refund_returns_succeeded() {
    let gw = mock_payment_gateway_impl();
    let refund = Refund::full("pay-refund-1", RefundReason::RequestedByCustomer);
    let result = gw.refund(refund).await.unwrap();
    assert_eq!(result.status, RefundStatus::Succeeded);
}
