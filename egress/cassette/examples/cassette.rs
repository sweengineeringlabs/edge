//! Minimal usage: load the SWE baseline and build a cassette
//! layer bound to a named fixture file.

fn main() {
    match swe_edge_egress_cassette::builder() {
        Err(e) => println!("swe_edge_egress_cassette: baseline parse failed: {e}"),
        Ok(b) => match b.build("example_cassette") {
            Ok(_) => println!("swe_edge_egress_cassette layer built (fixture: example_cassette.yaml)"),
            Err(e) => println!("swe_edge_egress_cassette: {e}"),
        },
    }
}
