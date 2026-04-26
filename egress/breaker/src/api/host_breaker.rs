//! Per-host breaker state abstraction — counterpart for `core::host_breaker`.
//!
//! The concrete `HostBreaker` state machine lives in `core::host_breaker`;
//! it implements [`CircuitBreakerNode`](crate::api::traits::CircuitBreakerNode)
//! and drives Closed → Open → HalfOpen state transitions.
