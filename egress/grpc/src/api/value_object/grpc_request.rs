//! gRPC request envelope.

use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::grpc_metadata::GrpcMetadata;

/// A gRPC request envelope.
///
/// ## Mandatory deadline
///
/// Every gRPC call MUST carry a per-call deadline.  Production gRPC
/// without deadlines deadlocks on slow servers or stuck connections —
/// the deadline is therefore a *required* positional argument of
/// [`GrpcRequest::new`], not an `Option`.  Construction without a
/// `Duration` is a compile error.
///
/// ## Optional cancellation
///
/// A caller-supplied [`CancellationToken`] can be attached via
/// [`GrpcRequest::with_cancellation`].  When the token fires, the
/// in-flight request is aborted by the egress transport.  Default:
/// no token (caller never cancels).
#[derive(Debug, Clone)]
pub struct GrpcRequest {
    /// Fully-qualified gRPC method path (e.g. `"pkg.Service/Method"`).
    pub method:       String,
    /// Raw protobuf-encoded request bytes.
    pub body:         Vec<u8>,
    /// Request metadata (headers).
    pub metadata:     GrpcMetadata,
    /// Per-call deadline.  Always present.
    pub deadline:     Duration,
    /// Optional caller-supplied cancellation token.
    pub cancellation: Option<CancellationToken>,
}

impl GrpcRequest {
    /// Create a new request for `method` with a raw protobuf `body` and a
    /// mandatory per-call `deadline`.
    ///
    /// `deadline` is a required positional argument — there is no overload
    /// without it and no default.  Compile error if omitted.
    pub fn new(
        method:   impl Into<String>,
        body:     Vec<u8>,
        deadline: Duration,
    ) -> Self {
        Self {
            method:       method.into(),
            body,
            metadata:     GrpcMetadata::default(),
            deadline,
            cancellation: None,
        }
    }

    /// Override the entire metadata block.
    pub fn with_metadata(mut self, metadata: GrpcMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a single metadata header key/value pair.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.headers.insert(name.into(), value.into());
        self
    }

    /// Attach a caller-supplied cancellation token.  Firing the token
    /// aborts the in-flight request from the caller's side.
    pub fn with_cancellation(mut self, token: CancellationToken) -> Self {
        self.cancellation = Some(token);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: GrpcRequest::new — deadline is stored.
    #[test]
    fn test_new_stores_method_body_and_deadline() {
        let d   = Duration::from_secs(5);
        let req = GrpcRequest::new("pkg.Svc/Method", vec![0xAB], d);
        assert_eq!(req.method,   "pkg.Svc/Method");
        assert_eq!(req.body,     vec![0xAB]);
        assert_eq!(req.deadline, d);
        assert!(req.metadata.headers.is_empty());
        assert!(req.cancellation.is_none());
    }

    /// @covers: GrpcRequest::with_header — single header insertion.
    #[test]
    fn test_with_header_inserts_single_metadata_entry() {
        let req = GrpcRequest::new("svc/M", vec![], Duration::from_secs(1))
            .with_header("authorization", "Bearer tok");
        assert_eq!(
            req.metadata.headers.get("authorization").map(String::as_str),
            Some("Bearer tok")
        );
    }

    /// @covers: GrpcRequest::with_metadata — full metadata replacement.
    #[test]
    fn test_with_metadata_replaces_metadata_entirely() {
        let meta = GrpcMetadata {
            headers: [("k".to_string(), "v".to_string())].into_iter().collect(),
        };
        let req = GrpcRequest::new("svc/M", vec![], Duration::from_secs(1))
            .with_metadata(meta);
        assert_eq!(req.metadata.headers.get("k").map(String::as_str), Some("v"));
    }

    /// @covers: GrpcRequest::with_cancellation — token is stored.
    #[test]
    fn test_with_cancellation_attaches_token() {
        let token = CancellationToken::new();
        let req   = GrpcRequest::new("svc/M", vec![], Duration::from_secs(1))
            .with_cancellation(token.clone());
        let stored = req.cancellation.as_ref().expect("token should be Some");
        assert!(!stored.is_cancelled());
        token.cancel();
        assert!(stored.is_cancelled(), "stored token must observe cancellation");
    }

    /// @covers: GrpcRequest — fields hold what was assigned.
    #[test]
    fn test_grpc_request_holds_method_body_and_deadline_via_struct_init() {
        let req = GrpcRequest {
            method:       "svc/Method".into(),
            body:         vec![1, 2],
            metadata:     GrpcMetadata::default(),
            deadline:     Duration::from_millis(250),
            cancellation: None,
        };
        assert_eq!(req.method, "svc/Method");
        assert_eq!(req.body,   vec![1, 2]);
        assert_eq!(req.deadline, Duration::from_millis(250));
    }
}
