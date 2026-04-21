//! Minimal usage: build the middleware layer. Scaffold phase:
//! `build()` returns NotImplemented. Once implemented, the
//! returned layer plugs into a `reqwest_middleware::ClientBuilder`.

fn main() {
    match swe_http_rate::builder().build() {
        Ok(_) => println!("swe_http_rate layer built"),
        Err(e) => println!("swe_http_rate: {e}"),
    }
}
