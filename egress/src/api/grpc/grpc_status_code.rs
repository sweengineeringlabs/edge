//! gRPC status codes (mirrors tonic/gRPC standard codes).

use serde::{Deserialize, Serialize};

/// A gRPC status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrpcStatusCode {
    Ok, Cancelled, Unknown, InvalidArgument, DeadlineExceeded, NotFound,
    AlreadyExists, PermissionDenied, ResourceExhausted, FailedPrecondition,
    Aborted, OutOfRange, Unimplemented, Internal, Unavailable, DataLoss, Unauthenticated,
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
        assert_ne!(GrpcStatusCode::Ok, GrpcStatusCode::Internal);
    }
}
