//! Value-object types for the reflection wire schema.
//!
//! These mirror the proto3 messages from `grpc/reflection/v1alpha/reflection.proto`.
//! We do NOT depend on `prost` or `tonic-build`; the codecs in
//! [`crate::core::wire`] hand-encode exactly the field set we need.
//!
//! Keeping the types small and exhaustively matched (no `_ => ` fallthroughs in
//! the codec) means that an unrecognised oneof variant is surfaced loudly at
//! compile time rather than swallowed at runtime.

use serde::{Deserialize, Serialize};

/// `ServerReflectionRequest.message_request` oneof.
///
/// Variants we don't yet implement (`FileContainingExtension`,
/// `AllExtensionNumbersOfType`) are still represented so the dispatcher
/// can return a structured error response instead of panicking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflectionRequest {
    /// `file_by_filename = 3` — request the FileDescriptorProto whose
    /// `name` is the given path (e.g. `"google/protobuf/empty.proto"`).
    FileByFilename(String),
    /// `file_containing_symbol = 4` — request the FileDescriptorProto
    /// that defines the fully-qualified symbol (e.g. `"pkg.MyService"`).
    FileContainingSymbol(String),
    /// `file_containing_extension = 5` — extension lookups are not yet
    /// implemented; surfacing the variant lets the dispatcher answer
    /// with `ErrorResponse(NOT_FOUND)` rather than dropping the request.
    FileContainingExtension {
        /// Containing message type, fully qualified.
        containing_type: String,
        /// Extension number.
        extension_number: i32,
    },
    /// `all_extension_numbers_of_type = 6` — also unimplemented.
    AllExtensionNumbersOfType(String),
    /// `list_services = 7` — return every registered service name.
    /// The string payload is ignored by reference servers; we mirror
    /// that and treat any value (including empty) as "list everything".
    ListServices(String),
    /// Request whose oneof was either absent or carried an unknown
    /// field number — we record it and answer with `INVALID_ARGUMENT`.
    Unknown,
}

/// Subset of `ServerReflectionResponse.message_response` we generate.
///
/// `ListServicesResponse` and `FileDescriptorResponse` are the two paths
/// grpcurl exercises by default; `ErrorResponse` covers everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflectionResponse {
    /// Successful list-services response — one `ServiceResponse` per name.
    ListServices(Vec<String>),
    /// Successful file-descriptor response — one or more raw
    /// `FileDescriptorProto` byte buffers.
    FileDescriptor(Vec<Vec<u8>>),
    /// Structured error response — `error_code` is a gRPC status code.
    Error {
        /// gRPC status code (numeric).
        error_code: i32,
        /// Human-readable error message.
        error_message: String,
    },
}

/// Snapshot of a registered descriptor — paired with the source bytes
/// of a `FileDescriptorProto`.
///
/// The reflection service stores one `Descriptor` per registered file;
/// `filename` and `symbols` are pre-extracted at registration time so
/// per-request lookup is O(1) under a read lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Descriptor {
    /// `FileDescriptorProto.name` (e.g. `"pkg/foo.proto"`).
    pub filename: String,
    /// Fully-qualified symbols defined in this file —
    /// `pkg.Service`, `pkg.Service.Method`, `pkg.Message`, etc.
    pub symbols: Vec<String>,
    /// Raw `FileDescriptorProto` bytes — what reflection clients receive verbatim.
    pub bytes: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: ReflectionRequest derives PartialEq for direct test assertions.
    #[test]
    fn test_reflection_request_partial_eq_compares_file_by_filename() {
        let a = ReflectionRequest::FileByFilename("a.proto".into());
        let b = ReflectionRequest::FileByFilename("a.proto".into());
        let c = ReflectionRequest::FileByFilename("b.proto".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    /// @covers: ReflectionResponse Error variant carries code + message.
    #[test]
    fn test_reflection_response_error_variant_carries_code_and_message() {
        let err = ReflectionResponse::Error {
            error_code: 5,
            error_message: "not found".into(),
        };
        match err {
            ReflectionResponse::Error { error_code, error_message } => {
                assert_eq!(error_code, 5);
                assert_eq!(error_message, "not found");
            }
            _ => panic!("expected Error variant"),
        }
    }

    /// @covers: Descriptor — round-trips simple data.
    #[test]
    fn test_descriptor_round_trips_basic_fields() {
        let d = Descriptor {
            filename: "pkg/foo.proto".into(),
            symbols: vec!["pkg.Foo".into(), "pkg.Foo.Bar".into()],
            bytes: vec![1, 2, 3],
        };
        assert_eq!(d.filename, "pkg/foo.proto");
        assert_eq!(d.symbols.len(), 2);
        assert_eq!(d.bytes, vec![1, 2, 3]);
    }
}
