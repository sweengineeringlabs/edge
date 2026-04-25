//! Minimal usage: load the SWE baseline (pass-through) and
//! apply to a fresh `reqwest::Client::builder()`.

fn main() {
    match swe_http_tls::builder() {
        Err(e) => println!("swe_http_tls: baseline parse failed: {e}"),
        Ok(b) => {
            println!("swe_http_tls: config loaded: {:?}", b.config());
            match b.build() {
                Ok(layer) => match layer.apply_to(reqwest::Client::builder()) {
                    Ok(_builder) => println!("swe_http_tls layer applied to ClientBuilder"),
                    Err(e) => println!("swe_http_tls: apply_to failed: {e}"),
                },
                Err(e) => println!("swe_http_tls: build failed: {e}"),
            }
        }
    }
}
