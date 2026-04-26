//! Minimal usage: load the SWE baseline and build the layer.
//! Scaffold phase: `build()` returns NotImplemented.

fn main() {
    match swe_edge_egress_retry::builder() {
        Err(e) => println!("swe_edge_egress_retry: baseline parse failed: {e}"),
        Ok(b) => match b.build() {
            Ok(_) => println!("swe_edge_egress_retry layer built"),
            Err(e) => println!("swe_edge_egress_retry: {e}"),
        },
    }
}
