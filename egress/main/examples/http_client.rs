//! HTTP outbound client — plain_http_outbound against a local mock server.
//!
//! Run:
//!     cargo run -p swe-edge-egress --example http_client
//!
//! Spins up a wiremock server, makes GET and POST calls through
//! plain_http_outbound, then prints the responses and exits.

use swe_edge_egress_http::{plain_http_outbound, HttpConfig, HttpOutbound, HttpRequest};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── local mock server ─────────────────────────────────────────────────────
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Hello from mock!"))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/items"))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header("content-type", "application/json")
                .set_body_string(r#"{"id":1,"name":"widget"}"#),
        )
        .mount(&server)
        .await;

    // ── build a plain (no-middleware) HTTP client ─────────────────────────────
    let client = plain_http_outbound(HttpConfig::with_base_url(server.uri()))?;

    // GET /hello
    let resp = client.get("/hello").await?;
    println!(
        "GET /hello  → {} | {}",
        resp.status,
        resp.text().unwrap_or_default()
    );

    // POST /items with a JSON body
    let req = HttpRequest::post("/items")
        .with_json(&serde_json::json!({"name": "widget"}))?;
    let resp = client.send(req).await?;
    println!(
        "POST /items → {} | {}",
        resp.status,
        resp.text().unwrap_or_default()
    );

    // health_check fires a GET to base_url and asserts 2xx
    match client.health_check().await {
        Ok(())  => println!("health_check  → ok"),
        Err(e)  => println!("health_check  → failed: {e}"),
    }

    Ok(())
}
