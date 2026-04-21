//! Minimal usage: load the SWE baseline and build the layer.
//! Scaffold phase: `build()` returns NotImplemented.

fn main() {
    match swe_http_cache::builder() {
        Err(e) => println!("swe_http_cache: baseline parse failed: {e}"),
        Ok(b) => match b.build() {
            Ok(_) => println!("swe_http_cache layer built"),
            Err(e) => println!("swe_http_cache: {e}"),
        },
    }
}
