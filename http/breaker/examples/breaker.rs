//! Minimal usage: load the SWE baseline and build the layer.
//! Scaffold phase: `build()` returns NotImplemented.

fn main() {
    match swe_edge_http_breaker::builder() {
        Err(e) => println!("swe_edge_http_breaker: baseline parse failed: {e}"),
        Ok(b) => match b.build() {
            Ok(_) => println!("swe_edge_http_breaker layer built"),
            Err(e) => println!("swe_edge_http_breaker: {e}"),
        },
    }
}
