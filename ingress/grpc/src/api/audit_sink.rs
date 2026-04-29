//! Audit sink — callback contract for per-dispatch audit events.
//!
//! [`AuditSink`] is invoked by [`crate::TonicGrpcServer`] for every
//! dispatched call AFTER any authorisation interceptor has run.  When
//! the server is configured with `allow_unauthenticated = true`,
//! events still fire — the `identity` field will simply be `None`.
//!
//! ## Why a separate sink (and not just structured logs)?
//!
//! Audit events have stricter retention/integrity requirements than
//! ordinary logs.  Wiring them through a dedicated trait lets
//! deployments forward the events to a tamper-evident store (Kafka,
//! S3 Object Lock, an internal SIEM) without coupling that pipeline to
//! the tracing macro stack.

use std::time::SystemTime;

use crate::api::value_object::GrpcStatusCode;

/// A single audit event emitted once per dispatched gRPC call.
///
/// The fields are intentionally minimal — sinks that need richer
/// context can pull it off [`crate::GrpcRequest::metadata`] from the
/// surrounding interceptor chain and inject it into their own pipeline.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Timestamp captured at dispatch time.
    pub timestamp:   SystemTime,
    /// Fully-qualified gRPC method path
    /// (e.g. `"/pkg.Service/Method"`).
    pub method:      String,
    /// Caller identity — `None` when the request was accepted under
    /// `allow_unauthenticated = true`.  Otherwise carries the
    /// fully-qualified principal name set by the authn / authz chain.
    pub identity:    Option<String>,
    /// Final gRPC status code returned to the wire.
    pub status:      GrpcStatusCode,
    /// Wall-clock duration of the dispatch in milliseconds.
    pub duration_ms: u64,
}

/// Receiver of [`AuditEvent`]s.
///
/// Implementations MUST be cheap and non-blocking — the server calls
/// `record` on the dispatch path; a slow sink will back up request
/// processing.  Real implementations typically push the event onto an
/// in-memory channel and let a background task drain it.
pub trait AuditSink: Send + Sync {
    /// Record one audit event.  Implementations must not panic.
    fn record(&self, event: AuditEvent);
}

/// Default no-op sink — drops every event.
///
/// Used by [`crate::TonicGrpcServer::new`] when the caller does not
/// supply an explicit sink.  Production deployments should replace
/// this with a real implementation.
pub struct NoopAuditSink;

impl AuditSink for NoopAuditSink {
    fn record(&self, _event: AuditEvent) {}
}

impl Default for NoopAuditSink {
    fn default() -> Self { Self }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;

    use super::*;

    /// Capturing sink — records every event into a shared `Vec` for assertions.
    struct CapturingAuditSink {
        events: Arc<Mutex<Vec<AuditEvent>>>,
    }

    impl AuditSink for CapturingAuditSink {
        fn record(&self, event: AuditEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    /// @covers: AuditSink — trait is object-safe (used via dyn AuditSink).
    #[test]
    fn test_audit_sink_trait_is_object_safe() {
        fn _accept(_: &dyn AuditSink) {}
    }

    /// @covers: NoopAuditSink::record — never panics, drops events silently.
    #[test]
    fn test_noop_audit_sink_record_drops_events_silently() {
        let sink = NoopAuditSink;
        sink.record(AuditEvent {
            timestamp:   SystemTime::now(),
            method:      "/svc/M".into(),
            identity:    Some("alice".into()),
            status:      GrpcStatusCode::Ok,
            duration_ms: 1,
        });
    }

    /// @covers: AuditSink::record — capturing impl receives every event.
    #[test]
    fn test_capturing_sink_record_receives_every_event() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = CapturingAuditSink { events: events.clone() };
        sink.record(AuditEvent {
            timestamp:   SystemTime::UNIX_EPOCH,
            method:      "/svc/A".into(),
            identity:    Some("alice".into()),
            status:      GrpcStatusCode::Ok,
            duration_ms: 5,
        });
        sink.record(AuditEvent {
            timestamp:   SystemTime::UNIX_EPOCH,
            method:      "/svc/B".into(),
            identity:    None,
            status:      GrpcStatusCode::PermissionDenied,
            duration_ms: 7,
        });
        let captured = events.lock().unwrap().clone();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0].method, "/svc/A");
        assert_eq!(captured[0].status, GrpcStatusCode::Ok);
        assert_eq!(captured[1].method, "/svc/B");
        assert_eq!(captured[1].status, GrpcStatusCode::PermissionDenied);
        assert_eq!(captured[1].identity, None);
    }
}
