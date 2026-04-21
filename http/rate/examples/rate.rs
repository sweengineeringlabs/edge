//! Minimal usage: load the SWE baseline and build the layer.
//! Scaffold phase: `build()` returns NotImplemented.

fn main() {
    match swe_http_rate::builder() {
        Err(e) => println!("swe_http_rate: baseline parse failed: {e}"),
        Ok(b) => match b.build() {
            Ok(_) => println!("swe_http_rate layer built"),
            Err(e) => println!("swe_http_rate: {e}"),
        },
    }
}
