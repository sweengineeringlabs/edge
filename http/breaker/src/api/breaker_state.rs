//! State-machine types shared between the breaker api/ and core/ layers.

/// Decision returned when a new request arrives at the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    /// Pass through — record the outcome afterward.
    Proceed,
    /// Breaker is open — fail fast without calling upstream.
    RejectOpen,
}

/// Outcome of a dispatched request, as seen by the breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Success,
    Failure,
}
