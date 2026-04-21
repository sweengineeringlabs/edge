//! Minimal usage: build the middleware layer. Scaffold phase:
//! `build()` returns NotImplemented. Once implemented, the
//! returned layer plugs into a `reqwest_middleware::ClientBuilder`.

fn main() {
    match swe_http_breaker::builder().build() {
        Ok(_) => println!("swe_http_breaker layer built"),
        Err(e) => println!("swe_http_breaker: {e}"),
    }
}
