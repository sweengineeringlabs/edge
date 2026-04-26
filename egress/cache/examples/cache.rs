//! Minimal usage: load the SWE baseline and build the layer.
//! Scaffold phase: `build()` returns NotImplemented.

fn main() {
    match swe_edge_egress_cache::builder() {
        Err(e) => println!("swe_edge_egress_cache: baseline parse failed: {e}"),
        Ok(b) => match b.build() {
            Ok(_) => println!("swe_edge_egress_cache layer built"),
            Err(e) => println!("swe_edge_egress_cache: {e}"),
        },
    }
}
