//! Inbound gateway trait re-export hub.

/// Marks an inbound adapter as a full ingress source.
/// Implementors satisfy `InboundSource + Send + Sync`.
pub(crate) trait IngressAdapter: Send + Sync {}
