//! gRPC request envelope.

use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::grpc_metadata::GrpcMetadata;

/// A gRPC request envelope handed to a [`crate::api::port::GrpcInbound`] handler.
///
/// ## Mandatory deadline
///
/// Every inbound request carries a per-call deadline.  The server populates
/// it from the wire `grpc-timeout` header (or applies a server default when
/// the client did not send one).  Construction without a `Duration` is a
/// compile error — see [`GrpcRequest::new`].
///
/// ## Cancellation propagation
///
/// `cancellation` is a [`CancellationToken`] the server fires when the
/// client TCP/HTTP-2 stream is closed before the handler completes.
/// Long-running handlers SHOULD select on this token to abort early
/// instead of running to completion against a disconnected caller.
#[derive(Debug, Clone)]
pub struct GrpcRequest {
    /// Fully-qualified gRPC method path (e.g. `"/pkg.Service/Method"`).
    pub method:       String,
    /// Raw protobuf-encoded request bytes.
    pub body:         Vec<u8>,
    /// Request metadata (headers / trailers).
    pub metadata:     GrpcMetadata,
    /// Per-call deadline (parsed from `grpc-timeout` or server default).
    pub deadline:     Duration,
    /// Token fired when the underlying client stream is cancelled.
    pub cancellation: Option<CancellationToken>,
}

impl GrpcRequest {
    /// Create a new request with a mandatory per-call deadline.
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

    /// Replace the metadata block.
    pub fn with_metadata(mut self, metadata: GrpcMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Attach a cancellation token sourced from the surrounding HTTP/2 stream.
    pub fn with_cancellation(mut self, token: CancellationToken) -> Self {
        self.cancellation = Some(token);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: GrpcRequest::new — three-argument signature with deadline.
    #[test]
    fn test_new_stores_method_body_and_deadline() {
        let req = GrpcRequest::new("svc/Method", vec![1, 2], Duration::from_millis(250));
        assert_eq!(req.method,   "svc/Method");
        assert_eq!(req.body,     vec![1, 2]);
        assert_eq!(req.deadline, Duration::from_millis(250));
        assert!(req.cancellation.is_none());
    }

    /// @covers: GrpcRequest::with_cancellation — token observed on the stored field.
    #[test]
    fn test_with_cancellation_propagates_cancel_signal() {
        let token = CancellationToken::new();
        let req   = GrpcRequest::new("svc/M", vec![], Duration::from_secs(1))
            .with_cancellation(token.clone());
        let stored = req.cancellation.as_ref().expect("token should be stored");
        assert!(!stored.is_cancelled());
        token.cancel();
        assert!(stored.is_cancelled());
    }

    /// @covers: GrpcRequest — fields hold what was assigned via struct init.
    #[test]
    fn test_grpc_request_holds_method_and_body() {
        let req = GrpcRequest {
            method:       "svc/Method".into(),
            body:         vec![1, 2],
            metadata:     GrpcMetadata::default(),
            deadline:     Duration::from_secs(1),
            cancellation: None,
        };
        assert_eq!(req.method, "svc/Method");
        assert_eq!(req.body,   vec![1, 2]);
    }
}
