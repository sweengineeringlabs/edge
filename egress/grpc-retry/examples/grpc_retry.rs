//! Minimal usage: load the SWE baseline and inspect the resolved policy.
//!
//! Wrapping a real `GrpcOutbound` requires a transport (see
//! `swe-edge-egress-grpc`'s `TonicGrpcClient`); this example shows
//! only the configuration step that doesn't need a server.

fn main() {
    match swe_edge_egress_grpc_retry::builder() {
        Ok(b) => {
            let cfg = b.config();
            println!(
                "swe_edge_egress_grpc_retry baseline: \
                 max_attempts={}, initial_backoff_ms={}, multiplier={}, jitter={}, max_backoff_ms={}",
                cfg.max_attempts,
                cfg.initial_backoff_ms,
                cfg.backoff_multiplier,
                cfg.jitter_factor,
                cfg.max_backoff_ms,
            );
        }
        Err(e) => println!("swe_edge_egress_grpc_retry: baseline parse failed: {e}"),
    }
}
