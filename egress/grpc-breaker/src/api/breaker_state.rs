//! State-machine types shared between the breaker api/ and core/.

use std::time::Instant;

/// Concrete state of the breaker.
///
/// Public for observability — consumers can introspect the
/// current state via
/// [`GrpcBreakerClient::state`](crate::saf::GrpcBreakerClient::state).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    /// Traffic flows normally.  Failures are counted; at the
    /// configured threshold the breaker trips Open.
    Closed,

    /// All requests short-circuit with `Unavailable`.  After
    /// `cool_down`, the next request promotes to HalfOpen.
    /// `since` records when the breaker entered Open so the
    /// admit-side check can compute elapsed cool-down without
    /// reading a separate field.
    Open {
        /// Instant the breaker last entered Open state.
        since: Instant,
    },

    /// One or more probe requests are in flight.  Successes
    /// count toward `half_open_probe_count`; any failure
    /// returns to Open.
    HalfOpen,
}

/// Decision returned when a new request arrives at the breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    /// Pass through — record the outcome afterward.
    Proceed,
    /// Breaker is open — fail fast without calling the inner client.
    RejectOpen,
}

/// Outcome of a dispatched request, as seen by the breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Call returned `Ok` (or a non-breaker-failure error).
    Success,
    /// Call returned a result classified as a breaker failure
    /// (transport-level Unavailable, status Unavailable, or
    /// status/transport Internal).  See `api::failure_kind`.
    Failure,
}
