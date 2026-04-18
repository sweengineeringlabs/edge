//! End-to-end tests for HttpOutbound sub-trait.
//!
//! Exercises only the HttpOutbound operations through the SAF factory.
//! A wiremock MockServer stands in for the real HTTP endpoint so the tests
//! exercise the full outbound stack (request build → send → response parse)
//! deterministically.

use edge_gateway::prelude::*;
use edge_gateway::saf::http::HttpRequest;
use edge_gateway::saf;

#[cfg(feature = "reqwest")]
#[tokio::test]
async fn e2e_http_outbound_send_get_post_json() {
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    Mock::given(method("GET")).and(path("/health")).and(query_param("verbose", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"mock": true})))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/users"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/users"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server).await;

    let uri = server.uri();
    let client = saf::rest_client_with_base_url(&uri);

    // --- HttpOutbound: send (custom request) ---
    let custom_request = HttpRequest::get("/health")
        .with_header("Accept", "application/json")
        .with_query("verbose", "true");
    let send_resp = client.send(custom_request).await.unwrap();
    assert!(send_resp.is_success());
    let send_body: serde_json::Value = send_resp.json().unwrap();
    assert_eq!(send_body["mock"], true);

    // --- HttpOutbound: get ---
    let get_resp = client.get("/users").await.unwrap();
    assert!(get_resp.is_success());
    assert_eq!(get_resp.status, 200);

    // --- HttpOutbound: post_json ---
    let payload = serde_json::json!({
        "name": "Alice",
        "email": "alice@example.com",
        "role": "admin"
    });
    let post_resp = client.post_json("/users", payload).await.unwrap();
    assert!(post_resp.is_success());
}

#[cfg(feature = "reqwest")]
#[tokio::test]
async fn e2e_http_outbound_put_json_and_delete() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    Mock::given(method("POST")).and(path("/posts"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server).await;
    Mock::given(method("PUT")).and(path("/posts/1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server).await;
    Mock::given(method("DELETE")).and(path("/posts/1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server).await;

    let uri = server.uri();
    let client = saf::rest_client_with_base_url(&uri);

    // Simulate create -> update -> delete resource lifecycle via Outbound
    let create_body = serde_json::json!({"title": "Draft Post", "status": "draft"});
    let create_resp = client.post_json("/posts", create_body).await.unwrap();
    assert!(create_resp.is_success());

    let update_body = serde_json::json!({"title": "Published Post", "status": "published"});
    let put_resp = client.put_json("/posts/1", update_body).await.unwrap();
    assert!(put_resp.is_success());
    assert_eq!(put_resp.status, 200);

    let del_resp = client.delete("/posts/1").await.unwrap();
    assert!(del_resp.is_success());
    assert_eq!(del_resp.status, 200);
}

#[cfg(feature = "reqwest")]
#[tokio::test]
async fn e2e_http_outbound_sequential_api_workflow() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let mock_body = ResponseTemplate::new(200).set_body_json(serde_json::json!({"mock": true}));

    Mock::given(method("GET")).and(path("/orders"))
        .respond_with(mock_body.clone())
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/orders"))
        .respond_with(mock_body.clone())
        .mount(&server).await;
    Mock::given(method("PUT")).and(path("/orders/ord_1"))
        .respond_with(mock_body.clone())
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/orders/ord_1"))
        .respond_with(mock_body.clone())
        .mount(&server).await;
    Mock::given(method("DELETE")).and(path("/orders/ord_1"))
        .respond_with(mock_body)
        .mount(&server).await;

    let uri = server.uri();
    let client = saf::rest_client_with_base_url(&uri);

    // Multi-step outbound flow simulating a typical REST API interaction.
    let list_resp = client.get("/orders").await.unwrap();
    assert!(list_resp.is_success());

    let order_payload = serde_json::json!({
        "item_id": "sku-999",
        "quantity": 3,
        "currency": "USD"
    });
    let order_resp = client.post_json("/orders", order_payload).await.unwrap();
    assert!(order_resp.is_success());

    let update_payload = serde_json::json!({"quantity": 5});
    let update_resp = client.put_json("/orders/ord_1", update_payload).await.unwrap();
    assert!(update_resp.is_success());

    let fetch_resp = client.get("/orders/ord_1").await.unwrap();
    assert!(fetch_resp.is_success());

    let cancel_resp = client.delete("/orders/ord_1").await.unwrap();
    assert!(cancel_resp.is_success());

    // Every response should have been the mock JSON body.
    for resp in [list_resp, order_resp, update_resp, fetch_resp, cancel_resp] {
        let body: serde_json::Value = resp.json().unwrap();
        assert_eq!(body["mock"], true);
    }
}
