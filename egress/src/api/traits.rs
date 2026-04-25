//! Outbound gateway trait re-export hub.

/// Marks an outbound adapter as a full egress sink.
/// Implementors satisfy `OutboundSink + Send + Sync`.
pub(crate) trait EgressAdapter: Send + Sync {}
