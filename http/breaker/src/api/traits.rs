//! Primary trait re-export hub and trait definitions for `swe_http_breaker`.

pub(crate) type HttpBreakerTrait = dyn crate::api::http_breaker::HttpBreaker;

/// Contract for per-host circuit breaker state machines.
/// Implementors track failure counts and state transitions;
/// the middleware layer holds one node per host.
pub(crate) trait CircuitBreakerNode {
    /// Returns `true` when the breaker is in the Open state
    /// (all requests should be rejected without calling upstream).
    fn is_open(&self) -> bool;

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
