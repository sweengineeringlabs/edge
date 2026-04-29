//! Classification of [`GrpcOutboundError`] values into the
//! breaker's binary success/failure outcome.
//!
//! Single source of truth for which gRPC errors trip the breaker.
//!
//! ## Counted as failures
//!
//! - `GrpcOutboundError::Unavailable(_)`
//! - `GrpcOutboundError::ConnectionFailed(_)`
//! - `GrpcOutboundError::Internal(_)`
//! - `GrpcOutboundError::Status(Unavailable, _)`
//! - `GrpcOutboundError::Status(Internal, _)`
//!
//! ## NOT counted as failures
//!
//! - `Status(ResourceExhausted, _)` — rate-limit signal, not an
//!   unhealthy upstream.  Retry layer handles it.
//! - `Status(Unauthenticated, _)` / `Status(PermissionDenied, _)` —
//!   credentials are wrong; the upstream is fine.  Tripping the
//!   breaker on auth failures would shed traffic that has nothing
//!   to do with upstream health.
//! - `Status(NotFound, _)`, `InvalidArgument`, `FailedPrecondition`,
//!   etc. — caller-visible business errors.  Upstream working as
//!   intended.
//! - `Timeout(_)` — caller's own deadline, not a breaker condition.
//! - `Cancelled(_)` — caller cancelled.

use swe_edge_egress_grpc::{GrpcOutboundError, GrpcStatusCode};

use crate::api::breaker_state::Outcome;

/// Classify an outbound result into the breaker's outcome.
pub(crate) fn classify<T>(result: &Result<T, GrpcOutboundError>) -> Outcome {
    let err = match result {
        Ok(_)  => return Outcome::Success,
        Err(e) => e,
    };
    match err {
        GrpcOutboundError::Unavailable(_)
        | GrpcOutboundError::ConnectionFailed(_)
        | GrpcOutboundError::Internal(_) => Outcome::Failure,

        GrpcOutboundError::Status(code, _) => match code {
            GrpcStatusCode::Unavailable | GrpcStatusCode::Internal => Outcome::Failure,
            // Explicit non-failure variants — exhaustive so a
            // new variant on `GrpcStatusCode` causes a compile
            // error here, not a silent default.
            GrpcStatusCode::Ok
            | GrpcStatusCode::Cancelled
            | GrpcStatusCode::Unknown
            | GrpcStatusCode::InvalidArgument
            | GrpcStatusCode::DeadlineExceeded
            | GrpcStatusCode::NotFound
            | GrpcStatusCode::AlreadyExists
            | GrpcStatusCode::PermissionDenied
            | GrpcStatusCode::ResourceExhausted
            | GrpcStatusCode::FailedPrecondition
            | GrpcStatusCode::Aborted
            | GrpcStatusCode::OutOfRange
            | GrpcStatusCode::Unimplemented
            | GrpcStatusCode::DataLoss
            | GrpcStatusCode::Unauthenticated => Outcome::Success,
        },

        GrpcOutboundError::Timeout(_) | GrpcOutboundError::Cancelled(_) => Outcome::Success,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: classify — Ok is success.
    #[test]
    fn test_classify_ok_is_success() {
        let r: Result<i32, GrpcOutboundError> = Ok(1);
        assert_eq!(classify(&r), Outcome::Success);
    }

    /// @covers: classify — transport Unavailable counts as failure.
    #[test]
    fn test_classify_transport_unavailable_is_failure() {
        let r: Result<(), _> = Err(GrpcOutboundError::Unavailable("lb".into()));
        assert_eq!(classify(&r), Outcome::Failure);
    }

    /// @covers: classify — ConnectionFailed counts as failure.
    #[test]
    fn test_classify_connection_failed_is_failure() {
        let r: Result<(), _> = Err(GrpcOutboundError::ConnectionFailed("rst".into()));
        assert_eq!(classify(&r), Outcome::Failure);
    }

    /// @covers: classify — transport Internal counts as failure.
    #[test]
    fn test_classify_transport_internal_is_failure() {
        let r: Result<(), _> = Err(GrpcOutboundError::Internal("oops".into()));
        assert_eq!(classify(&r), Outcome::Failure);
    }

    /// @covers: classify — Status Unavailable counts as failure.
    #[test]
    fn test_classify_status_unavailable_is_failure() {
        let r: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::Unavailable, "down".into(),
        ));
        assert_eq!(classify(&r), Outcome::Failure);
    }

    /// @covers: classify — Status Internal counts as failure.
    #[test]
    fn test_classify_status_internal_is_failure() {
        let r: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::Internal, "bug".into(),
        ));
        assert_eq!(classify(&r), Outcome::Failure);
    }

    /// @covers: classify — auth failures do NOT trip the breaker.
    #[test]
    fn test_classify_auth_failures_are_success() {
        let r1: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::Unauthenticated, "x".into(),
        ));
        let r2: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::PermissionDenied, "y".into(),
        ));
        assert_eq!(classify(&r1), Outcome::Success);
        assert_eq!(classify(&r2), Outcome::Success);
    }

    /// @covers: classify — ResourceExhausted does NOT trip breaker.
    #[test]
    fn test_classify_resource_exhausted_is_success() {
        let r: Result<(), _> = Err(GrpcOutboundError::Status(
            GrpcStatusCode::ResourceExhausted, "quota".into(),
        ));
        assert_eq!(classify(&r), Outcome::Success);
    }

    /// @covers: classify — Timeout does NOT trip breaker.
    #[test]
    fn test_classify_timeout_is_success() {
        let r: Result<(), _> = Err(GrpcOutboundError::Timeout("d".into()));
        assert_eq!(classify(&r), Outcome::Success);
    }

    /// @covers: classify — Cancelled does NOT trip breaker.
    #[test]
    fn test_classify_cancelled_is_success() {
        let r: Result<(), _> = Err(GrpcOutboundError::Cancelled("t".into()));
        assert_eq!(classify(&r), Outcome::Success);
    }

    /// @covers: classify — business-error statuses are success.
    #[test]
    fn test_classify_business_error_statuses_are_success() {
        for code in [
            GrpcStatusCode::NotFound,
            GrpcStatusCode::InvalidArgument,
            GrpcStatusCode::AlreadyExists,
            GrpcStatusCode::FailedPrecondition,
            GrpcStatusCode::Aborted,
            GrpcStatusCode::OutOfRange,
            GrpcStatusCode::Unimplemented,
            GrpcStatusCode::DataLoss,
            GrpcStatusCode::Unknown,
            GrpcStatusCode::Cancelled,
            GrpcStatusCode::DeadlineExceeded,
        ] {
            let r: Result<(), _> = Err(GrpcOutboundError::Status(code, "x".into()));
            assert_eq!(
                classify(&r),
                Outcome::Success,
                "{code:?} should not count as a breaker failure",
            );
        }
    }
}
