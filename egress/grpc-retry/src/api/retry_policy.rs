//! Per-status retry decision.  Single source of truth for which
//! gRPC status codes the decorator will retry.
//!
//! The policy is hand-written, not config-driven, because the
//! retryable set is a property of gRPC's semantic contract — not
//! a tunable knob.  Specifically:
//!
//! - `Unavailable` and `ResourceExhausted` are retryable per the
//!   gRPC retry whitepaper (the latter with a longer backoff,
//!   surfaced via [`RetryDecision::is_resource_exhausted`]).
//! - `Unauthenticated` and `PermissionDenied` MUST NOT be
//!   retried — a bad token won't become good by trying again,
//!   and silent retries hide auth failures from the caller.
//! - `DeadlineExceeded` must not be retried — the caller's
//!   deadline already counts the retry budget; re-issuing
//!   guarantees a second deadline trip.
//! - `Internal` is not retried — server bug, retrying just
//!   amplifies the bug and burns the deadline.

use swe_edge_egress_grpc::{GrpcOutboundError, GrpcStatusCode};

/// Decision returned by [`classify`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    /// Treat as success — return to caller, no retry.
    Success,
    /// Retry-eligible failure with the standard backoff schedule.
    Retry,
    /// Retry-eligible failure that should use a *longer* backoff
    /// (`ResourceExhausted` typically reflects rate-limit or
    /// quota pressure on the server; backing off harder gives
    /// the server room to recover).
    RetryWithLongerBackoff,
    /// Terminal failure — surface to caller without retrying.
    Terminal,
}

impl RetryDecision {
    /// True if the decision indicates the call should be re-issued.
    pub fn should_retry(self) -> bool {
        matches!(
            self,
            RetryDecision::Retry | RetryDecision::RetryWithLongerBackoff,
        )
    }

    /// True if the decision is the longer-backoff variant.
    pub fn is_resource_exhausted(self) -> bool {
        matches!(self, RetryDecision::RetryWithLongerBackoff)
    }
}

/// Classify an outbound result into a retry decision.
///
/// Mapping table (for non-`Ok` outcomes):
///
/// | Variant                                            | Decision                  |
/// |----------------------------------------------------|---------------------------|
/// | `Status(Unavailable, _)` / `Unavailable(_)`        | `Retry`                   |
/// | `Status(ResourceExhausted, _)`                     | `RetryWithLongerBackoff`  |
/// | `Status(Unauthenticated, _)`                       | `Terminal`                |
/// | `Status(PermissionDenied, _)`                      | `Terminal`                |
/// | `Status(DeadlineExceeded, _)` / `Timeout(_)`       | `Terminal`                |
/// | `Status(Internal, _)` / `Internal(_)`              | `Terminal`                |
/// | `ConnectionFailed(_)`                              | `Retry`                   |
/// | `Cancelled(_)` / `Status(Cancelled, _)`            | `Terminal`                |
/// | other `Status(_, _)`                               | `Terminal`                |
///
/// `ConnectionFailed` is treated as `Retry` because it's a
/// transport-level transient (DNS hiccup, TCP RST during a
/// rolling deploy) and matches the canonical `Unavailable`
/// gRPC semantics from the caller's perspective.
pub fn classify<T>(result: &Result<T, GrpcOutboundError>) -> RetryDecision {
    let err = match result {
        Ok(_)  => return RetryDecision::Success,
        Err(e) => e,
    };
    match err {
        GrpcOutboundError::Status(code, _) => match code {
            GrpcStatusCode::Unavailable        => RetryDecision::Retry,
            GrpcStatusCode::ResourceExhausted  => RetryDecision::RetryWithLongerBackoff,
            // Explicit non-retryable variants — listed here so
            // adding a new variant on `GrpcStatusCode` would
            // surface as a missing arm, not a silent default.
            GrpcStatusCode::Unauthenticated
            | GrpcStatusCode::PermissionDenied
            | GrpcStatusCode::DeadlineExceeded
            | GrpcStatusCode::Internal
            | GrpcStatusCode::Cancelled
            | GrpcStatusCode::Ok
            | GrpcStatusCode::Unknown
            | GrpcStatusCode::InvalidArgument
            | GrpcStatusCode::NotFound
            | GrpcStatusCode::AlreadyExists
            | GrpcStatusCode::FailedPrecondition
            | GrpcStatusCode::Aborted
            | GrpcStatusCode::OutOfRange
            | GrpcStatusCode::Unimplemented
            | GrpcStatusCode::DataLoss => RetryDecision::Terminal,
        },
        GrpcOutboundError::ConnectionFailed(_) => RetryDecision::Retry,
        GrpcOutboundError::Unavailable(_)      => RetryDecision::Retry,
        GrpcOutboundError::Timeout(_)
        | GrpcOutboundError::Internal(_)
        | GrpcOutboundError::Cancelled(_) => RetryDecision::Terminal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: classify — Unavailable status retries.
    #[test]
    fn test_classify_status_unavailable_returns_retry() {
        let r: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::Unavailable, "lb".into(),
        ));
        assert_eq!(classify(&r), RetryDecision::Retry);
    }

    /// @covers: classify — ResourceExhausted retries with longer backoff.
    #[test]
    fn test_classify_status_resource_exhausted_returns_longer_backoff() {
        let r: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::ResourceExhausted, "quota".into(),
        ));
        assert_eq!(classify(&r), RetryDecision::RetryWithLongerBackoff);
        assert!(classify(&r).is_resource_exhausted());
    }

    /// @covers: classify — PermissionDenied is NEVER retried.
    #[test]
    fn test_classify_status_permission_denied_is_terminal() {
        let r: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::PermissionDenied, "no".into(),
        ));
        assert_eq!(classify(&r), RetryDecision::Terminal);
        assert!(!classify(&r).should_retry());
    }

    /// @covers: classify — Unauthenticated is NEVER retried.
    #[test]
    fn test_classify_status_unauthenticated_is_terminal() {
        let r: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::Unauthenticated, "bad token".into(),
        ));
        assert_eq!(classify(&r), RetryDecision::Terminal);
        assert!(!classify(&r).should_retry());
    }

    /// @covers: classify — DeadlineExceeded is terminal (caller's
    /// deadline already counts retry budget).
    #[test]
    fn test_classify_status_deadline_exceeded_is_terminal() {
        let r: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::DeadlineExceeded, "tick".into(),
        ));
        assert_eq!(classify(&r), RetryDecision::Terminal);
    }

    /// @covers: classify — Internal is terminal (server bug).
    #[test]
    fn test_classify_status_internal_is_terminal() {
        let r: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::Internal, "bug".into(),
        ));
        assert_eq!(classify(&r), RetryDecision::Terminal);
    }

    /// @covers: classify — ConnectionFailed retries.
    #[test]
    fn test_classify_connection_failed_retries() {
        let r: Result<(), _> = Err(GrpcOutboundError::ConnectionFailed("rst".into()));
        assert_eq!(classify(&r), RetryDecision::Retry);
    }

    /// @covers: classify — Timeout is terminal.
    #[test]
    fn test_classify_timeout_is_terminal() {
        let r: Result<(), _> = Err(GrpcOutboundError::Timeout("deadline".into()));
        assert_eq!(classify(&r), RetryDecision::Terminal);
    }

    /// @covers: classify — Cancelled is terminal (caller's choice).
    #[test]
    fn test_classify_cancelled_is_terminal() {
        let r: Result<(), _> = Err(GrpcOutboundError::Cancelled("token".into()));
        assert_eq!(classify(&r), RetryDecision::Terminal);
    }

    /// @covers: classify — Ok is success, not retry.
    #[test]
    fn test_classify_ok_returns_success() {
        let r: Result<i32, GrpcOutboundError> = Ok(42);
        assert_eq!(classify(&r), RetryDecision::Success);
        assert!(!classify(&r).should_retry());
    }

    /// @covers: should_retry — Retry and RetryWithLongerBackoff are retryable.
    #[test]
    fn test_should_retry_true_for_retry_variants() {
        assert!(RetryDecision::Retry.should_retry());
        assert!(RetryDecision::RetryWithLongerBackoff.should_retry());
        assert!(!RetryDecision::Success.should_retry());
        assert!(!RetryDecision::Terminal.should_retry());
    }

    /// @covers: classify — every non-retry status code is terminal.
    /// Ensures the explicit-arms guard above stays in sync if a
    /// variant gets renamed.
    #[test]
    fn test_classify_non_retry_status_codes_all_terminal() {
        for code in [
            GrpcStatusCode::Cancelled,
            GrpcStatusCode::Unknown,
            GrpcStatusCode::InvalidArgument,
            GrpcStatusCode::NotFound,
            GrpcStatusCode::AlreadyExists,
            GrpcStatusCode::FailedPrecondition,
            GrpcStatusCode::Aborted,
            GrpcStatusCode::OutOfRange,
            GrpcStatusCode::Unimplemented,
            GrpcStatusCode::DataLoss,
        ] {
            let r: Result<(), _> = Err(GrpcOutboundError::Status(code, "x".into()));
            assert_eq!(
                classify(&r),
                RetryDecision::Terminal,
                "expected {code:?} to be Terminal",
            );
        }
    }
}
