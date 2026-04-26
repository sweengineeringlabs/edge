//! Minimal usage: load the SWE baseline (pass-through) and
//! apply to a fresh `reqwest::Client::builder()`.

use swe_edge_egress_tls::TlsApplier;

fn main() {
    match swe_edge_egress_tls::builder() {
        Err(e) => println!("swe_edge_egress_tls: baseline parse failed: {e}"),
        Ok(b) => {
            println!("swe_edge_egress_tls: config loaded: {:?}", b.config());
            match b.build() {
                Ok(layer) => match layer.apply_to(reqwest::Client::builder()) {
                    Ok(_builder) => println!("swe_edge_egress_tls layer applied to ClientBuilder"),
                    Err(e) => println!("swe_edge_egress_tls: apply_to failed: {e}"),
                },
                Err(e) => println!("swe_edge_egress_tls: build failed: {e}"),
            }
        }
    }
}
