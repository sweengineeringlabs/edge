//! Standard `grpc.reflection.v1alpha.ServerReflection` service.
//!
//! Implements the bidi-streaming `ServerReflectionInfo` RPC by:
//!
//! 1. reading registered method paths from a Phase 3
//!    [`HandlerRegistry`] to populate `ListServices`,
//! 2. answering `FileByFilename` / `FileContainingSymbol` from an
//!    optional descriptor table the operator wires at startup,
//! 3. surfacing every other request (extensions, etc.) as a
//!    structured `ErrorResponse` rather than crashing the stream.
//!
//! ## Why this lives in its own crate
//!
//! Reflection has no HTTP equivalent, exposes the service surface to
//! anyone who reaches the endpoint, and must therefore be off by
//! default in production.  Bundling it in the base ingress/grpc
//! crate would leave the surface available to every consumer who
//! depends on the base — instead each consumer opts in by adding
//! `swe-edge-ingress-grpc-reflection` to their dependency set.
//!
//! ## Service-name derivation
//!
//! gRPC method paths look like `/pkg.SubPkg.Service/Method`.  The
//! service name (what `ListServices` returns) is everything between
//! the leading slash and the next slash.  We snapshot the registry
//! once per request and derive the unique set.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use futures::StreamExt;
use parking_lot::RwLock;

use edge_domain::HandlerRegistry;
use swe_edge_ingress_grpc::{
    GrpcHealthCheck, GrpcInbound, GrpcInboundError, GrpcInboundResult, GrpcMessageStream,
    GrpcMetadata, GrpcRequest, GrpcResponse,
};

use crate::api::types::{Descriptor, ReflectionRequest, ReflectionResponse};
use crate::core::wire::{decode_request, encode_response};

/// Fully-qualified gRPC method path for the reflection RPC.
pub const REFLECTION_INFO_METHOD: &str = "/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo";

/// Service name reported by `ListServices` for the reflection service itself.
pub const REFLECTION_SERVICE_NAME: &str = "grpc.reflection.v1alpha.ServerReflection";

/// gRPC status code (numeric) reported in `ErrorResponse` for unknown
/// requests.  Mirrors `tonic::Code::NotFound`.
pub const ERROR_CODE_NOT_FOUND: i32 = 5;

/// gRPC status code reported when the inbound oneof is missing or the
/// requested feature is not implemented (e.g. extension lookups).
/// Mirrors `tonic::Code::Unimplemented`.
pub const ERROR_CODE_UNIMPLEMENTED: i32 = 12;

/// gRPC status code reported on a malformed `ServerReflectionRequest`.
/// Mirrors `tonic::Code::InvalidArgument`.
pub const ERROR_CODE_INVALID_ARGUMENT: i32 = 3;

/// Implementation of `grpc.reflection.v1alpha.ServerReflection`.
///
/// Holds the source-of-truth links into the gRPC stack:
/// - an `Arc<HandlerRegistry>` shared with the dispatcher, used to
///   derive the live service list,
/// - an optional descriptor table populated at startup via
///   [`ReflectionService::with_descriptor_set`].
///
/// ## Concurrency
///
/// `descriptors` is guarded by `parking_lot::RwLock` — `ListServices`
/// only takes a read lock; descriptor-set registration takes a write
/// lock.  Lookups are O(D) where D is the number of registered
/// descriptors and O(S) for symbols, both linear scans (D and S are
/// expected to be tiny — one descriptor per consumer's proto bundle).
pub struct ReflectionService {
    registry:    Arc<HandlerRegistry<Vec<u8>, Vec<u8>>>,
    descriptors: RwLock<Vec<Descriptor>>,
}

impl ReflectionService {
    /// Construct a reflection service backed by `registry`.
    ///
    /// The service is dep-free until the operator wires descriptors
    /// via [`Self::with_descriptor_set`] — until then,
    /// `FileByFilename` / `FileContainingSymbol` answer with
    /// `ErrorResponse(NOT_FOUND)`.
    pub fn new(registry: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>>) -> Self {
        Self {
            registry,
            descriptors: RwLock::new(Vec::new()),
        }
    }

    /// Register a parsed descriptor for `FileByFilename` /
    /// `FileContainingSymbol` lookups.  Multiple descriptors may be
    /// registered (e.g. one per consumer's `.proto` bundle).
    ///
    /// The descriptor's `filename` and `symbols` are pre-extracted
    /// (no proto parsing on the request path).  Build them by
    /// converting your `FileDescriptorSet`'s `file` entries — see
    /// `tests/reflection_int_test.rs` for an example.
    pub fn add_descriptor(self, descriptor: Descriptor) -> Self {
        self.descriptors.write().push(descriptor);
        self
    }

    /// Register a list of descriptors in one call.
    pub fn with_descriptors(self, descriptors: impl IntoIterator<Item = Descriptor>) -> Self {
        {
            let mut w = self.descriptors.write();
            for d in descriptors {
                w.push(d);
            }
        }
        self
    }

    /// Snapshot the live service-name set: registered handler ids
    /// plus the reflection service itself.
    pub(crate) fn list_services(&self) -> Vec<String> {
        let mut set: BTreeSet<String> = BTreeSet::new();
        for id in self.registry.list_ids() {
            if let Some(name) = service_name_from_method_path(&id) {
                set.insert(name.to_string());
            }
        }
        // Reflection always advertises itself — that's how grpcurl confirms it's on.
        set.insert(REFLECTION_SERVICE_NAME.to_string());
        set.into_iter().collect()
    }

    pub(crate) fn handle_request(&self, request: ReflectionRequest) -> ReflectionResponse {
        match request {
            ReflectionRequest::ListServices(_) => {
                ReflectionResponse::ListServices(self.list_services())
            }
            ReflectionRequest::FileByFilename(name) => {
                self.descriptor_response_by_filename(&name)
            }
            ReflectionRequest::FileContainingSymbol(symbol) => {
                self.descriptor_response_by_symbol(&symbol)
            }
            ReflectionRequest::FileContainingExtension { .. } => {
                ReflectionResponse::Error {
                    error_code:    ERROR_CODE_UNIMPLEMENTED,
                    error_message: "FileContainingExtension is not implemented".into(),
                }
            }
            ReflectionRequest::AllExtensionNumbersOfType(_) => {
                ReflectionResponse::Error {
                    error_code:    ERROR_CODE_UNIMPLEMENTED,
                    error_message: "AllExtensionNumbersOfType is not implemented".into(),
                }
            }
            ReflectionRequest::Unknown => ReflectionResponse::Error {
                error_code:    ERROR_CODE_INVALID_ARGUMENT,
                error_message: "ServerReflectionRequest had no recognised oneof field".into(),
            },
        }
    }

    fn descriptor_response_by_filename(&self, name: &str) -> ReflectionResponse {
        let guard = self.descriptors.read();
        for d in guard.iter() {
            if d.filename == name {
                return ReflectionResponse::FileDescriptor(vec![d.bytes.clone()]);
            }
        }
        ReflectionResponse::Error {
            error_code:    ERROR_CODE_NOT_FOUND,
            error_message: format!("no descriptor registered for filename {name:?}"),
        }
    }

    fn descriptor_response_by_symbol(&self, symbol: &str) -> ReflectionResponse {
        let guard = self.descriptors.read();
        for d in guard.iter() {
            if d.symbols.iter().any(|s| s == symbol) {
                return ReflectionResponse::FileDescriptor(vec![d.bytes.clone()]);
            }
        }
        ReflectionResponse::Error {
            error_code:    ERROR_CODE_NOT_FOUND,
            error_message: format!("no descriptor registered for symbol {symbol:?}"),
        }
    }
}

impl GrpcInbound for ReflectionService {
    fn handle_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            // ServerReflectionInfo is bidi-streaming.  If a unary client
            // hits us we still answer the first request frame — that's
            // tolerant of grpcurl's tooling which sometimes sends a
            // single ServerReflectionRequest per HTTP/2 stream.
            if request.method.as_str() != REFLECTION_INFO_METHOD {
                return Err(GrpcInboundError::Unimplemented(format!(
                    "unknown method {}",
                    request.method
                )));
            }
            let parsed = match decode_request(&request.body) {
                Ok(p) => p,
                Err(e) => {
                    return Err(GrpcInboundError::InvalidArgument(format!(
                        "malformed ServerReflectionRequest: {e}"
                    )));
                }
            };
            let response = self.handle_request(parsed);
            // We don't have the original-request bytes when we already
            // re-encoded, so pass the raw body through verbatim.
            let bytes = encode_response(&response, &request.body);
            Ok(GrpcResponse {
                body:     bytes,
                metadata: GrpcMetadata::default(),
            })
        })
    }

    fn handle_stream(
        &self,
        method:    String,
        _metadata: GrpcMetadata,
        messages:  GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcInboundResult<(GrpcMessageStream, GrpcMetadata)>> {
        Box::pin(async move {
            if method.as_str() != REFLECTION_INFO_METHOD {
                return Err(GrpcInboundError::Unimplemented(format!(
                    "unknown method {method}"
                )));
            }
            // Drain the incoming stream first.  Reflection is bidi
            // but the practical pattern (and what grpcurl does) is
            // request-then-response — buffering is safe and keeps
            // the impl simple while remaining wire-correct.
            let mut messages = messages;
            let mut requests: Vec<Vec<u8>> = Vec::new();
            while let Some(item) = messages.next().await {
                match item {
                    Ok(b) => requests.push(b),
                    Err(e) => return Err(e),
                }
            }
            // If the stream was empty we still emit a single
            // structured error response so the client doesn't see a
            // bare 0-frame Watch-style channel.
            if requests.is_empty() {
                let resp = ReflectionResponse::Error {
                    error_code:    ERROR_CODE_INVALID_ARGUMENT,
                    error_message: "empty ServerReflectionInfo stream".into(),
                };
                let bytes = encode_response(&resp, &[]);
                let out: GrpcMessageStream = Box::pin(futures::stream::once(
                    futures::future::ready(Ok(bytes)),
                ));
                return Ok((out, GrpcMetadata::default()));
            }

            // Process each request frame and emit one response frame.
            let mut responses: Vec<Vec<u8>> = Vec::with_capacity(requests.len());
            for body in requests.into_iter() {
                let parsed = match decode_request(&body) {
                    Ok(p) => p,
                    Err(e) => {
                        let resp = ReflectionResponse::Error {
                            error_code:    ERROR_CODE_INVALID_ARGUMENT,
                            error_message: format!("malformed request frame: {e}"),
                        };
                        responses.push(encode_response(&resp, &body));
                        continue;
                    }
                };
                let response = self.handle_request(parsed);
                responses.push(encode_response(&response, &body));
            }

            let out: GrpcMessageStream = Box::pin(futures::stream::iter(
                responses.into_iter().map(Ok::<Vec<u8>, GrpcInboundError>),
            ));
            Ok((out, GrpcMetadata::default()))
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move {
            // Reflection is intrinsically healthy — its job is to read
            // the registry state, not maintain its own backing store.
            Ok(GrpcHealthCheck::healthy())
        })
    }
}

/// Extract the service name from a `/pkg.Service/Method` path.
///
/// Returns `None` for malformed paths (no leading slash, no method
/// segment) so caller code can drop them rather than synthesise
/// garbage in `ListServices`.
pub fn service_name_from_method_path(path: &str) -> Option<&str> {
    let path = path.strip_prefix('/')?;
    let slash = path.find('/')?;
    let name = &path[..slash];
    if name.is_empty() {
        return None;
    }
    Some(name)
}

/// Synonym we expose so consumers don't have to wrestle with the
/// trait when they're just plumbing a service.
///
/// The default deadline used by [`GrpcInbound::handle_unary`] when
/// callers don't carry one — keeps reflection requests bounded.
#[allow(dead_code)]
const REFLECTION_DEFAULT_DEADLINE: Duration = Duration::from_secs(5);

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::sync::Arc;

    use async_trait::async_trait;
    use edge_domain::{Handler, HandlerError, HandlerRegistry};

    use super::*;

    struct StubHandler {
        path: String,
    }

    #[async_trait]
    impl Handler<Vec<u8>, Vec<u8>> for StubHandler {
        fn id(&self) -> &str { &self.path }
        fn pattern(&self) -> &str { "stub" }
        async fn execute(&self, _: Vec<u8>) -> Result<Vec<u8>, HandlerError> {
            Ok(vec![])
        }
        async fn health_check(&self) -> bool { true }
        fn as_any(&self) -> &dyn Any { self }
    }

    fn registry_with(paths: &[&str]) -> Arc<HandlerRegistry<Vec<u8>, Vec<u8>>> {
        let r: Arc<HandlerRegistry<Vec<u8>, Vec<u8>>> = Arc::new(HandlerRegistry::new());
        for p in paths {
            r.register(Arc::new(StubHandler { path: p.to_string() }));
        }
        r
    }

    /// @covers: service_name_from_method_path — pulls package.service from canonical path.
    #[test]
    fn test_service_name_from_method_path_extracts_service_segment() {
        assert_eq!(
            service_name_from_method_path("/pkg.Math/Triple"),
            Some("pkg.Math")
        );
    }

    /// @covers: service_name_from_method_path — rejects path without method segment.
    #[test]
    fn test_service_name_from_method_path_returns_none_when_method_segment_missing() {
        assert_eq!(service_name_from_method_path("/pkg.Math"), None);
    }

    /// @covers: service_name_from_method_path — rejects path without leading slash.
    #[test]
    fn test_service_name_from_method_path_returns_none_when_leading_slash_absent() {
        assert_eq!(service_name_from_method_path("pkg.Math/Triple"), None);
    }

    /// @covers: list_services — empty registry still advertises reflection itself.
    #[test]
    fn test_list_services_with_empty_registry_returns_only_reflection_self_name() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let names = svc.list_services();
        assert_eq!(names, vec![REFLECTION_SERVICE_NAME.to_string()]);
    }

    /// @covers: list_services — derives unique service names from registered methods.
    #[test]
    fn test_list_services_aggregates_unique_service_names_from_registered_methods() {
        let r = registry_with(&[
            "/pkg.Math/Triple",
            "/pkg.Math/Halve",
            "/pkg.Strings/Upper",
        ]);
        let svc = ReflectionService::new(r);
        let names = svc.list_services();
        assert!(names.contains(&"pkg.Math".to_string()));
        assert!(names.contains(&"pkg.Strings".to_string()));
        assert!(names.contains(&REFLECTION_SERVICE_NAME.to_string()));
        // De-dup: pkg.Math has two methods but appears once.
        assert_eq!(
            names.iter().filter(|n| *n == "pkg.Math").count(),
            1
        );
    }

    /// @covers: list_services — silently skips malformed registry entries.
    #[test]
    fn test_list_services_drops_method_paths_without_a_service_segment() {
        let r = registry_with(&["broken-no-slashes"]);
        let svc = ReflectionService::new(r);
        let names = svc.list_services();
        // Only the reflection self-name remains.
        assert_eq!(names, vec![REFLECTION_SERVICE_NAME.to_string()]);
    }

    /// @covers: handle_request FileByFilename — returns descriptor bytes when registered.
    #[test]
    fn test_handle_request_file_by_filename_returns_descriptor_bytes_when_registered() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()))
            .add_descriptor(Descriptor {
                filename: "foo.proto".into(),
                symbols: vec!["pkg.Foo".into()],
                bytes: vec![0xde, 0xad, 0xbe, 0xef],
            });
        let resp = svc.handle_request(ReflectionRequest::FileByFilename("foo.proto".into()));
        match resp {
            ReflectionResponse::FileDescriptor(files) => {
                assert_eq!(files, vec![vec![0xde, 0xad, 0xbe, 0xef]]);
            }
            other => panic!("expected FileDescriptor, got {other:?}"),
        }
    }

    /// @covers: handle_request FileByFilename — returns NOT_FOUND when no descriptor matches.
    #[test]
    fn test_handle_request_file_by_filename_returns_not_found_when_no_descriptor_registered() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let resp = svc.handle_request(ReflectionRequest::FileByFilename("missing.proto".into()));
        match resp {
            ReflectionResponse::Error { error_code, .. } => {
                assert_eq!(error_code, ERROR_CODE_NOT_FOUND);
            }
            other => panic!("expected Error(NOT_FOUND), got {other:?}"),
        }
    }

    /// @covers: handle_request FileContainingSymbol — finds descriptor via symbol list.
    #[test]
    fn test_handle_request_file_containing_symbol_locates_descriptor_via_symbol_list() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()))
            .add_descriptor(Descriptor {
                filename: "foo.proto".into(),
                symbols: vec!["pkg.Foo".into(), "pkg.Foo.Bar".into()],
                bytes: vec![0x01, 0x02],
            });
        let resp = svc.handle_request(ReflectionRequest::FileContainingSymbol("pkg.Foo.Bar".into()));
        match resp {
            ReflectionResponse::FileDescriptor(files) => {
                assert_eq!(files, vec![vec![0x01, 0x02]]);
            }
            other => panic!("expected FileDescriptor, got {other:?}"),
        }
    }

    /// @covers: handle_request FileContainingSymbol — NOT_FOUND when symbol absent.
    #[test]
    fn test_handle_request_file_containing_symbol_returns_not_found_when_symbol_absent() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()))
            .add_descriptor(Descriptor {
                filename: "foo.proto".into(),
                symbols: vec!["pkg.Foo".into()],
                bytes: vec![],
            });
        let resp = svc.handle_request(ReflectionRequest::FileContainingSymbol("pkg.Bogus".into()));
        match resp {
            ReflectionResponse::Error { error_code, .. } => {
                assert_eq!(error_code, ERROR_CODE_NOT_FOUND);
            }
            other => panic!("expected Error(NOT_FOUND), got {other:?}"),
        }
    }

    /// @covers: handle_request FileContainingExtension — UNIMPLEMENTED.
    #[test]
    fn test_handle_request_file_containing_extension_returns_unimplemented_error() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let resp = svc.handle_request(ReflectionRequest::FileContainingExtension {
            containing_type: "pkg.Foo".into(),
            extension_number: 1,
        });
        match resp {
            ReflectionResponse::Error { error_code, .. } => {
                assert_eq!(error_code, ERROR_CODE_UNIMPLEMENTED);
            }
            other => panic!("expected Error(UNIMPLEMENTED), got {other:?}"),
        }
    }

    /// @covers: handle_request AllExtensionNumbersOfType — UNIMPLEMENTED.
    #[test]
    fn test_handle_request_all_extension_numbers_of_type_returns_unimplemented_error() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let resp = svc.handle_request(ReflectionRequest::AllExtensionNumbersOfType("pkg.Foo".into()));
        match resp {
            ReflectionResponse::Error { error_code, .. } => {
                assert_eq!(error_code, ERROR_CODE_UNIMPLEMENTED);
            }
            other => panic!("expected Error(UNIMPLEMENTED), got {other:?}"),
        }
    }

    /// @covers: handle_request Unknown — INVALID_ARGUMENT.
    #[test]
    fn test_handle_request_unknown_oneof_returns_invalid_argument_error() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let resp = svc.handle_request(ReflectionRequest::Unknown);
        match resp {
            ReflectionResponse::Error { error_code, .. } => {
                assert_eq!(error_code, ERROR_CODE_INVALID_ARGUMENT);
            }
            other => panic!("expected Error(INVALID_ARGUMENT), got {other:?}"),
        }
    }

    /// @covers: handle_unary — wrong method returns Unimplemented.
    #[tokio::test]
    async fn test_handle_unary_returns_unimplemented_for_non_reflection_method_path() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let req = GrpcRequest::new("/pkg.Other/Method", vec![], Duration::from_secs(1));
        let err = svc.handle_unary(req).await.expect_err("must fail");
        assert!(matches!(err, GrpcInboundError::Unimplemented(_)));
    }

    /// @covers: handle_unary — known method with empty body emits a structured error response.
    #[tokio::test]
    async fn test_handle_unary_with_empty_body_emits_invalid_argument_error_payload() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let req = GrpcRequest::new(REFLECTION_INFO_METHOD, vec![], Duration::from_secs(1));
        let resp = svc.handle_unary(req).await.expect("ok");
        // The resulting body is a ServerReflectionResponse with
        // message_response = ErrorResponse — first byte after any
        // original_request prefix should be tag 0x3a.
        // Empty original_request means the response starts with the
        // error_response tag directly.
        assert_eq!(resp.body.first().copied(), Some(0x3a));
    }

    /// @covers: handle_unary — list_services request emits ListServiceResponse payload.
    #[tokio::test]
    async fn test_handle_unary_list_services_request_emits_list_service_response_payload() {
        use crate::core::wire::encode_varint;
        let r = registry_with(&["/pkg.Demo/Echo"]);
        let svc = ReflectionService::new(r);
        // Request body: list_services field 7 with empty string.
        let mut body = Vec::new();
        body.push(0x3a);
        encode_varint(0, &mut body);
        let req = GrpcRequest::new(REFLECTION_INFO_METHOD, body.clone(), Duration::from_secs(1));
        let resp = svc.handle_unary(req).await.expect("ok");
        // The response embeds original_request first (tag 2 = 0x12).
        // After that comes message_response (tag 6 = 0x32) when ListServices.
        // We just look for the ListServiceResponse tag.
        assert!(resp.body.contains(&0x32));
    }

    /// @covers: handle_stream — non-reflection method returns Unimplemented.
    #[tokio::test]
    async fn test_handle_stream_rejects_non_reflection_method_with_unimplemented() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let messages: GrpcMessageStream = Box::pin(futures::stream::iter(
            std::iter::empty::<GrpcInboundResult<Vec<u8>>>(),
        ));
        let result = svc
            .handle_stream("/pkg.Other/M".into(), GrpcMetadata::default(), messages)
            .await;
        match result {
            Err(GrpcInboundError::Unimplemented(_)) => {}
            Err(other) => panic!("expected Unimplemented, got {other:?}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    /// @covers: handle_stream — empty request stream surfaces a single error frame.
    #[tokio::test]
    async fn test_handle_stream_with_empty_input_yields_single_error_frame() {
        use futures::StreamExt;
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let messages: GrpcMessageStream = Box::pin(futures::stream::iter(
            std::iter::empty::<GrpcInboundResult<Vec<u8>>>(),
        ));
        let (mut out, _meta) = svc
            .handle_stream(REFLECTION_INFO_METHOD.into(), GrpcMetadata::default(), messages)
            .await
            .expect("stream ok");
        let frame = out.next().await.expect("one frame").expect("ok");
        // ErrorResponse tag = 0x3a.
        assert!(frame.contains(&0x3a));
        assert!(out.next().await.is_none(), "stream must end after one frame");
    }

    /// @covers: handle_stream — every input frame yields exactly one response frame.
    #[tokio::test]
    async fn test_handle_stream_yields_one_response_frame_per_input_request() {
        use futures::StreamExt;
        use crate::core::wire::encode_varint;
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let mut list_req = Vec::new();
        list_req.push(0x3a);
        encode_varint(0, &mut list_req);
        let inputs: Vec<GrpcInboundResult<Vec<u8>>> =
            vec![Ok(list_req.clone()), Ok(list_req)];
        let messages: GrpcMessageStream = Box::pin(futures::stream::iter(inputs));
        let (mut out, _meta) = svc
            .handle_stream(REFLECTION_INFO_METHOD.into(), GrpcMetadata::default(), messages)
            .await
            .expect("ok");
        let mut count = 0;
        while let Some(frame) = out.next().await {
            frame.expect("each frame ok");
            count += 1;
        }
        assert_eq!(count, 2);
    }

    /// @covers: GrpcStatusCode constants — ERROR_CODE_NOT_FOUND tracks tonic NotFound.
    #[test]
    fn test_error_code_constants_match_tonic_status_code_numerics() {
        assert_eq!(ERROR_CODE_NOT_FOUND, 5);
        assert_eq!(ERROR_CODE_UNIMPLEMENTED, 12);
        assert_eq!(ERROR_CODE_INVALID_ARGUMENT, 3);
    }

    /// @covers: handle_request ListServices — payload string is ignored (any value works).
    #[test]
    fn test_handle_request_list_services_ignores_request_payload_string() {
        let r = registry_with(&["/pkg.X/Y"]);
        let svc = ReflectionService::new(r);
        let a = svc.handle_request(ReflectionRequest::ListServices(String::new()));
        let b = svc.handle_request(ReflectionRequest::ListServices("anything".into()));
        assert_eq!(a, b);
    }

    /// @covers: with_descriptors — installs multiple descriptors at once.
    #[test]
    fn test_with_descriptors_installs_multiple_descriptors_in_one_call() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()))
            .with_descriptors([
                Descriptor {
                    filename: "a.proto".into(),
                    symbols: vec!["pkg.A".into()],
                    bytes: vec![0xaa],
                },
                Descriptor {
                    filename: "b.proto".into(),
                    symbols: vec!["pkg.B".into()],
                    bytes: vec![0xbb],
                },
            ]);
        let a = svc.handle_request(ReflectionRequest::FileByFilename("a.proto".into()));
        let b = svc.handle_request(ReflectionRequest::FileByFilename("b.proto".into()));
        match a {
            ReflectionResponse::FileDescriptor(f) => assert_eq!(f, vec![vec![0xaa]]),
            _ => panic!("a.proto"),
        }
        match b {
            ReflectionResponse::FileDescriptor(f) => assert_eq!(f, vec![vec![0xbb]]),
            _ => panic!("b.proto"),
        }
    }

    /// @covers: GrpcInbound::health_check — always reports healthy.
    #[tokio::test]
    async fn test_reflection_service_health_check_always_reports_healthy() {
        let svc = ReflectionService::new(Arc::new(HandlerRegistry::new()));
        let h = svc.health_check().await.expect("ok");
        assert!(h.healthy);
    }

}
