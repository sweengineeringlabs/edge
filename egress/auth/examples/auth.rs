//! Minimal usage: load the SWE baseline (pass-through) and
//! build the layer. Scaffold phase: `build()` returns
//! NotImplemented.

fn main() {
    match swe_edge_egress_auth::builder() {
        Err(e) => println!("swe_edge_egress_auth: baseline parse failed: {e}"),
        Ok(b) => {
            println!("swe_edge_egress_auth: config loaded: {:?}", b.config());
            match b.build() {
                Ok(_) => println!("swe_edge_egress_auth layer built"),
                Err(e) => println!("swe_edge_egress_auth: {e}"),
            }
        }
    }
}
