//! Primary trait re-export hub and trait definitions for `swe_edge_egress_breaker`.

/// Contract for per-host circuit breaker state machines.
/// Implementors track failure counts and state transitions;
/// the middleware layer holds one node per host.
pub(crate) trait CircuitBreakerNode {
    /// Called BEFORE dispatching a request. Returns whether to
    /// proceed or reject fast; may promote Open → HalfOpen.
    fn admit(
        &mut self,
        config: &crate::api::breaker_config::BreakerConfig,
    ) -> crate::api::breaker_state::Admission;

    /// Called AFTER dispatching a request that `admit` approved.
    /// Updates internal state based on outcome.
    fn record(
        &mut self,
        config: &crate::api::breaker_config::BreakerConfig,
        outcome: crate::api::breaker_state::Outcome,
    );
}
