//! Integration tests for the HTTP outbound domain.

use swe_edge_egress_http::{HttpAuth, HttpMethod, HttpRequest, HttpResponse};

/// @covers: HttpRequest — constructor helpers create correct method variants.
#[test]
fn test_http_request_get_creates_get_method() {
    let req = HttpRequest::get("https://example.com/api");
    assert_eq!(req.method, HttpMethod::Get);
    assert_eq!(req.url, "https://example.com/api");
}

/// @covers: HttpRequest::post — POST variant is distinct from GET.
#[test]
fn test_http_request_post_creates_post_method() {
    let req = HttpRequest::post("/api/data");
    assert_eq!(req.method, HttpMethod::Post);
}

/// @covers: HttpResponse::is_success — 2xx range returns true.
#[test]
fn test_http_response_is_success_for_200() {
    let resp = HttpResponse::new(200, b"ok".to_vec());
    assert!(resp.is_success());
    assert!(!resp.is_client_error());
    assert!(!resp.is_server_error());
}

/// @covers: HttpAuth::bearer — creates bearer auth.
#[test]
fn test_http_auth_bearer_stores_token() {
    let auth = HttpAuth::bearer("tok_abc");
    assert!(matches!(auth, HttpAuth::Bearer { token } if token == "tok_abc"));
}
