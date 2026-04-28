//! Exhaustive bidirectional mapping between [`GrpcStatusCode`], `tonic::Code`,
//! and the gRPC wire status integer (RFC: <https://grpc.io/docs/guides/status-codes/>).
//!
//! All 17 standard gRPC status codes are covered.  Adding or removing a
//! variant on either side will fail to compile here, which is the whole
//! point — this module is a single source of truth.

use crate::api::value_object::GrpcStatusCode;

/// Convert a [`tonic::Code`] (server-side / wire-side enum) into the
/// crate-local [`GrpcStatusCode`].  Total — covers all 17 variants.
pub fn from_tonic_code(code: tonic::Code) -> GrpcStatusCode {
    match code {
        tonic::Code::Ok                 => GrpcStatusCode::Ok,
        tonic::Code::Cancelled          => GrpcStatusCode::Cancelled,
        tonic::Code::Unknown            => GrpcStatusCode::Unknown,
        tonic::Code::InvalidArgument    => GrpcStatusCode::InvalidArgument,
        tonic::Code::DeadlineExceeded   => GrpcStatusCode::DeadlineExceeded,
        tonic::Code::NotFound           => GrpcStatusCode::NotFound,
        tonic::Code::AlreadyExists      => GrpcStatusCode::AlreadyExists,
        tonic::Code::PermissionDenied   => GrpcStatusCode::PermissionDenied,
        tonic::Code::ResourceExhausted  => GrpcStatusCode::ResourceExhausted,
        tonic::Code::FailedPrecondition => GrpcStatusCode::FailedPrecondition,
        tonic::Code::Aborted            => GrpcStatusCode::Aborted,
        tonic::Code::OutOfRange         => GrpcStatusCode::OutOfRange,
        tonic::Code::Unimplemented      => GrpcStatusCode::Unimplemented,
        tonic::Code::Internal           => GrpcStatusCode::Internal,
        tonic::Code::Unavailable        => GrpcStatusCode::Unavailable,
        tonic::Code::DataLoss           => GrpcStatusCode::DataLoss,
        tonic::Code::Unauthenticated    => GrpcStatusCode::Unauthenticated,
    }
}

/// Convert a crate-local [`GrpcStatusCode`] into a [`tonic::Code`].  Total.
pub fn to_tonic_code(code: GrpcStatusCode) -> tonic::Code {
    match code {
        GrpcStatusCode::Ok                 => tonic::Code::Ok,
        GrpcStatusCode::Cancelled          => tonic::Code::Cancelled,
        GrpcStatusCode::Unknown            => tonic::Code::Unknown,
        GrpcStatusCode::InvalidArgument    => tonic::Code::InvalidArgument,
        GrpcStatusCode::DeadlineExceeded   => tonic::Code::DeadlineExceeded,
        GrpcStatusCode::NotFound           => tonic::Code::NotFound,
        GrpcStatusCode::AlreadyExists      => tonic::Code::AlreadyExists,
        GrpcStatusCode::PermissionDenied   => tonic::Code::PermissionDenied,
        GrpcStatusCode::ResourceExhausted  => tonic::Code::ResourceExhausted,
        GrpcStatusCode::FailedPrecondition => tonic::Code::FailedPrecondition,
        GrpcStatusCode::Aborted            => tonic::Code::Aborted,
        GrpcStatusCode::OutOfRange         => tonic::Code::OutOfRange,
        GrpcStatusCode::Unimplemented      => tonic::Code::Unimplemented,
        GrpcStatusCode::Internal           => tonic::Code::Internal,
        GrpcStatusCode::Unavailable        => tonic::Code::Unavailable,
        GrpcStatusCode::DataLoss           => tonic::Code::DataLoss,
        GrpcStatusCode::Unauthenticated    => tonic::Code::Unauthenticated,
    }
}

/// Parse a numeric `grpc-status` wire value into a [`GrpcStatusCode`].
///
/// Returns [`GrpcStatusCode::Unknown`] for any value outside the standard
/// 0..=16 range — that matches the gRPC spec's "unrecognized code maps to Unknown"
/// rule and ensures the parser never panics on malformed servers.
pub fn from_wire(value: i32) -> GrpcStatusCode {
    from_tonic_code(tonic::Code::from(value))
}

/// Encode a [`GrpcStatusCode`] as the numeric `grpc-status` wire value.
pub fn to_wire(code: GrpcStatusCode) -> i32 {
    to_tonic_code(code) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All 17 variants enumerated for round-trip coverage.
    const ALL_17: [GrpcStatusCode; 17] = [
        GrpcStatusCode::Ok,
        GrpcStatusCode::Cancelled,
        GrpcStatusCode::Unknown,
        GrpcStatusCode::InvalidArgument,
        GrpcStatusCode::DeadlineExceeded,
        GrpcStatusCode::NotFound,
        GrpcStatusCode::AlreadyExists,
        GrpcStatusCode::PermissionDenied,
        GrpcStatusCode::ResourceExhausted,
        GrpcStatusCode::FailedPrecondition,
        GrpcStatusCode::Aborted,
        GrpcStatusCode::OutOfRange,
        GrpcStatusCode::Unimplemented,
        GrpcStatusCode::Internal,
        GrpcStatusCode::Unavailable,
        GrpcStatusCode::DataLoss,
        GrpcStatusCode::Unauthenticated,
    ];

    /// @covers: from_tonic_code, to_tonic_code — round-trip fidelity for all 17 variants.
    #[test]
    fn test_round_trip_through_tonic_code_preserves_all_17_variants() {
        for code in ALL_17 {
            let trip = from_tonic_code(to_tonic_code(code));
            assert_eq!(trip, code, "round-trip failed for {code:?}");
        }
    }

    /// @covers: to_wire, from_wire — round-trip via numeric wire value for all 17 variants.
    #[test]
    fn test_round_trip_through_wire_value_preserves_all_17_variants() {
        for code in ALL_17 {
            let wire = to_wire(code);
            let trip = from_wire(wire);
            assert_eq!(trip, code, "wire round-trip failed for {code:?} (wire={wire})");
        }
    }

    /// @covers: from_wire — out-of-range values normalize to Unknown without panic.
    #[test]
    fn test_from_wire_returns_unknown_for_out_of_range_value() {
        // 99 is not a defined gRPC status code.
        assert_eq!(from_wire(99), GrpcStatusCode::Unknown);
        // negative likewise.
        assert_eq!(from_wire(-1), GrpcStatusCode::Unknown);
    }

    /// @covers: to_wire — Ok maps to wire 0 and Unauthenticated to 16.
    #[test]
    fn test_to_wire_matches_canonical_grpc_codes() {
        assert_eq!(to_wire(GrpcStatusCode::Ok),               0);
        assert_eq!(to_wire(GrpcStatusCode::Cancelled),        1);
        assert_eq!(to_wire(GrpcStatusCode::Unknown),          2);
        assert_eq!(to_wire(GrpcStatusCode::InvalidArgument),  3);
        assert_eq!(to_wire(GrpcStatusCode::DeadlineExceeded), 4);
        assert_eq!(to_wire(GrpcStatusCode::Unauthenticated),  16);
    }
}
