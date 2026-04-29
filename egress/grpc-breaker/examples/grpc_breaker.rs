//! Minimal usage: load the SWE baseline and inspect the resolved policy.

fn main() {
    match swe_edge_egress_grpc_breaker::builder() {
        Ok(b) => {
            let cfg = b.config();
            println!(
                "swe_edge_egress_grpc_breaker baseline: \
                 failure_threshold={}, cool_down_seconds={}, half_open_probe_count={}",
                cfg.failure_threshold,
                cfg.cool_down_seconds,
                cfg.half_open_probe_count,
            );
        }
        Err(e) => println!("swe_edge_egress_grpc_breaker: baseline parse failed: {e}"),
    }
}
