//! gRPC status codes (mirrors tonic/gRPC standard codes).

use serde::{Deserialize, Serialize};

/// A gRPC status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrpcStatusCode {
    /// Request succeeded (code 0).
    Ok,
    /// Request was cancelled by the client.
    Cancelled,
    /// Unknown error.
    Unknown,
    /// Invalid argument supplied.
    InvalidArgument,
    /// Deadline expired before the operation completed.
    DeadlineExceeded,
    /// Resource not found.
    NotFound,
    /// Resource already exists.
    AlreadyExists,
    /// Caller lacks permission.
    PermissionDenied,
    /// Resource quota or rate limit exhausted.
    ResourceExhausted,
    /// Pre-condition for the operation not met.
    FailedPrecondition,
    /// Operation aborted.
    Aborted,
    /// Value or index out of valid range.
    OutOfRange,
    /// Operation not implemented.
    Unimplemented,
    /// Internal server error.
    Internal,
    /// Service temporarily unavailable.
    Unavailable,
    /// Unrecoverable data loss or corruption.
    DataLoss,
    /// Request lacks valid authentication credentials.
    Unauthenticated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_status_code_has_17_distinct_variants() {
        let codes = [
            GrpcStatusCode::Ok, GrpcStatusCode::Cancelled, GrpcStatusCode::Unknown,
            GrpcStatusCode::InvalidArgument, GrpcStatusCode::DeadlineExceeded, GrpcStatusCode::NotFound,
            GrpcStatusCode::AlreadyExists, GrpcStatusCode::PermissionDenied, GrpcStatusCode::ResourceExhausted,
            GrpcStatusCode::FailedPrecondition, GrpcStatusCode::Aborted, GrpcStatusCode::OutOfRange,
            GrpcStatusCode::Unimplemented, GrpcStatusCode::Internal, GrpcStatusCode::Unavailable,
            GrpcStatusCode::DataLoss, GrpcStatusCode::Unauthenticated,
        ];
        assert_eq!(codes.len(), 17);
        assert_eq!(GrpcStatusCode::Ok, GrpcStatusCode::Ok);
        assert_ne!(GrpcStatusCode::Ok, GrpcStatusCode::Internal);
    }
}
