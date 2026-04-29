//! Standard `grpc.health.v1.Health` service implementation.
//!
//! Implements the gRPC health-check protocol as documented at
//! <https://github.com/grpc/grpc/blob/master/doc/health-checking.md>.
//!
//! ## Wire schema (hand-encoded — no proto codegen needed)
//!
//! ```proto
//! package grpc.health.v1;
//!
//! message HealthCheckRequest  { string service = 1; }
//! message HealthCheckResponse { ServingStatus status = 1; }
//!
//! enum ServingStatus {
//!     UNKNOWN         = 0;
//!     SERVING         = 1;
//!     NOT_SERVING     = 2;
//!     SERVICE_UNKNOWN = 3; // used only by Watch
//! }
//!
//! service Health {
//!     rpc Check(HealthCheckRequest) returns (HealthCheckResponse);
//!     rpc Watch(HealthCheckRequest) returns (stream HealthCheckResponse);
//! }
//! ```
//!
//! ## Service-name convention
//!
//! - The empty string `""` denotes the **server overall** — its status is
//!   the AND of every registered service.
//! - Any other value names a single service registered with
//!   [`HealthService::set_status`].
//!
//! Per the protocol spec, an unknown service name returns
//! `tonic::Code::NotFound` for `Check` and a `SERVICE_UNKNOWN` payload
//! for `Watch`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use futures::StreamExt;
use parking_lot::RwLock;
use tokio::sync::broadcast;

use crate::api::port::grpc_inbound::{
    GrpcHealthCheck, GrpcInbound, GrpcInboundError, GrpcInboundResult, GrpcMessageStream,
};
use crate::api::value_object::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};

/// Wire-equivalent of `grpc.health.v1.HealthCheckResponse.ServingStatus`.
///
/// Numeric values are stable on the wire; new variants must extend the
/// list, never reorder it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ServingStatus {
    /// Unknown — server has no opinion (rarely returned by Check).
    Unknown        = 0,
    /// SERVING — service is healthy and accepting calls.
    Serving        = 1,
    /// NOT_SERVING — service is intentionally refusing calls.
    NotServing     = 2,
    /// SERVICE_UNKNOWN — only used in Watch responses for unregistered services.
    ServiceUnknown = 3,
}

/// Fully-qualified gRPC method path of `grpc.health.v1.Health.Check`.
pub const HEALTH_CHECK_METHOD: &str = "/grpc.health.v1.Health/Check";

/// Fully-qualified gRPC method path of `grpc.health.v1.Health.Watch`.
pub const HEALTH_WATCH_METHOD: &str = "/grpc.health.v1.Health/Watch";

/// Implementation of the standard `grpc.health.v1.Health` service.
///
/// Holds a per-service status table protected by a `parking_lot::RwLock`
/// and broadcasts every status update on a Tokio broadcast channel so
/// `Watch` subscribers receive a frame whenever the value changes.
///
/// ## Concurrency contract
///
/// - `set_status` and `set_overall_status` are O(1) under the write lock.
/// - `Check` is O(1) under the read lock.
/// - `Watch` keeps a long-lived broadcaster receiver — slow subscribers
///   that fall behind by `WATCH_CHANNEL_CAPACITY` updates will receive
///   `RecvError::Lagged` and the stream is then aborted with
///   `tonic::Code::ResourceExhausted` — matching how typical health
///   servers respond when consumers cannot keep up.
pub struct HealthService {
    statuses:   RwLock<HashMap<String, ServingStatus>>,
    /// Broadcaster fires `(service_name, status)` on every state change.
    broadcaster: broadcast::Sender<(String, ServingStatus)>,
}

/// Capacity of the broadcast channel used by `Watch`.
///
/// Historical default in `grpc-health-probe` and other reference impls
/// is small (8) — keeping it modest avoids unbounded memory pressure
/// while still tolerating short bursts of state changes.
pub const WATCH_CHANNEL_CAPACITY: usize = 16;

impl HealthService {
    /// Construct an empty service.  No services are registered; the
    /// overall service-name (`""`) is implicit and starts as
    /// [`ServingStatus::Serving`].
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(WATCH_CHANNEL_CAPACITY);
        let mut statuses = HashMap::new();
        statuses.insert(String::new(), ServingStatus::Serving);
        Self {
            statuses:    RwLock::new(statuses),
            broadcaster: tx,
        }
    }

    /// Set the status for a named service.  Use the empty string to
    /// drive the **server overall** status directly.
    pub fn set_status(&self, service: impl Into<String>, status: ServingStatus) {
        let service = service.into();
        {
            let mut guard = self.statuses.write();
            guard.insert(service.clone(), status);
        }
        // Best-effort fan-out — `send` returns Err only when there are
        // no live receivers, which is fine.
        let _ = self.broadcaster.send((service, status));
    }

    /// Convenience alias for `set_status("", status)` — drives the
    /// implicit server-overall service name.
    pub fn set_overall_status(&self, status: ServingStatus) {
        self.set_status(String::new(), status);
    }

    /// Look up the current status for a service.  Returns `None` when
    /// the service name is not registered.
    pub fn get_status(&self, service: &str) -> Option<ServingStatus> {
        self.statuses.read().get(service).copied()
    }

    /// Subscribe to status changes — every call to [`set_status`] from
    /// this point onward will be forwarded on the returned receiver.
    /// New subscribers do NOT see the historical value; the caller is
    /// expected to read the current status with [`get_status`] first.
    pub fn subscribe(&self) -> broadcast::Receiver<(String, ServingStatus)> {
        self.broadcaster.subscribe()
    }

    /// Drive the implicit overall status from a snapshot of every named
    /// service: SERVING iff *every* service is SERVING.
    ///
    /// Useful for callers that want a single line of code at startup or
    /// after a config reload to recompute the overall status.
    pub fn recompute_overall_status(&self) {
        let new_overall = {
            let guard = self.statuses.read();
            let all_named_serving = guard
                .iter()
                .filter(|(name, _)| !name.is_empty())
                .all(|(_, status)| *status == ServingStatus::Serving);
            if guard.iter().filter(|(name, _)| !name.is_empty()).count() == 0 {
                // Nothing registered yet — keep the default SERVING.
                ServingStatus::Serving
            } else if all_named_serving {
                ServingStatus::Serving
            } else {
                ServingStatus::NotServing
            }
        };
        self.set_overall_status(new_overall);
    }
}

impl Default for HealthService {
    fn default() -> Self { Self::new() }
}

impl GrpcInbound for HealthService {
    fn handle_unary(&self, request: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
        Box::pin(async move {
            match request.method.as_str() {
                HEALTH_CHECK_METHOD => self.handle_check(&request.body),
                HEALTH_WATCH_METHOD => Err(GrpcInboundError::Status(
                    GrpcStatusCode::Unimplemented,
                    "Watch must be invoked via streaming dispatch".into(),
                )),
                other => Err(GrpcInboundError::Unimplemented(format!(
                    "unknown method {other}"
                ))),
            }
        })
    }

    fn handle_stream(
        &self,
        method:    String,
        _metadata: GrpcMetadata,
        messages:  GrpcMessageStream,
    ) -> BoxFuture<'_, GrpcInboundResult<(GrpcMessageStream, GrpcMetadata)>> {
        Box::pin(async move {
            if method.as_str() != HEALTH_WATCH_METHOD {
                // Fall back to handle_unary for non-Watch methods (Check)
                // by reading the first frame and routing through it.
                let mut messages = messages;
                let body = match messages.next().await {
                    Some(Ok(b))  => b,
                    Some(Err(e)) => return Err(e),
                    None         => vec![],
                };
                let req = GrpcRequest::new(method, body, Duration::from_secs(30));
                let resp = self.handle_unary(req).await?;
                let out: GrpcMessageStream = Box::pin(futures::stream::once(
                    futures::future::ready(Ok(resp.body)),
                ));
                return Ok((out, resp.metadata));
            }

            // Watch path — read the first frame, decode the service name,
            // emit the current status, and then subscribe for changes.
            let mut messages = messages;
            let body = match messages.next().await {
                Some(Ok(b))  => b,
                Some(Err(e)) => return Err(e),
                None         => vec![],
            };
            let service = decode_health_check_request(&body).unwrap_or_default();
            let initial = self
                .get_status(&service)
                .unwrap_or(ServingStatus::ServiceUnknown);
            let rx = self.subscribe();
            let target_service = service.clone();

            // Build the streaming output as an `unfold`-driven stream.
            // First frame is the snapshot; subsequent frames push updates
            // for `target_service`.  Lagged subscribers abort the stream
            // with ResourceExhausted; sender-closed terminates cleanly.
            enum WatchPhase {
                Initial,
                Streaming,
                Closed,
            }

            struct WatchState {
                phase:    WatchPhase,
                rx:       broadcast::Receiver<(String, ServingStatus)>,
                target:   String,
                snapshot: ServingStatus,
            }

            let state = WatchState {
                phase:    WatchPhase::Initial,
                rx,
                target:   target_service,
                snapshot: initial,
            };

            let stream = futures::stream::unfold(state, |mut s| async move {
                match s.phase {
                    WatchPhase::Initial => {
                        let frame = encode_health_check_response(s.snapshot);
                        s.phase = WatchPhase::Streaming;
                        Some((Ok(frame), s))
                    }
                    WatchPhase::Streaming => loop {
                        match s.rx.recv().await {
                            Ok((svc, status)) => {
                                if svc == s.target {
                                    let frame = encode_health_check_response(status);
                                    return Some((Ok(frame), s));
                                }
                                // Different service — keep waiting.
                            }
                            Err(broadcast::error::RecvError::Lagged(_)) => {
                                s.phase = WatchPhase::Closed;
                                return Some((
                                    Err(GrpcInboundError::Status(
                                        GrpcStatusCode::ResourceExhausted,
                                        "watch subscriber fell behind".into(),
                                    )),
                                    s,
                                ));
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                return None;
                            }
                        }
                    },
                    WatchPhase::Closed => None,
                }
            });

            let boxed: GrpcMessageStream = Box::pin(stream);
            Ok((boxed, GrpcMetadata::default()))
        })
    }

    fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
        Box::pin(async move {
            // The health service itself is always healthy — its job is
            // to report on others.
            Ok(GrpcHealthCheck::healthy())
        })
    }
}

impl HealthService {
    fn handle_check(&self, body: &[u8]) -> GrpcInboundResult<GrpcResponse> {
        let service = decode_health_check_request(body).unwrap_or_default();
        match self.get_status(&service) {
            Some(status) => Ok(GrpcResponse {
                body:     encode_health_check_response(status),
                metadata: GrpcMetadata::default(),
            }),
            None => Err(GrpcInboundError::NotFound(format!(
                "service {service:?} not registered"
            ))),
        }
    }
}

/// Aggregate dispatcher — ties [`HealthService`] to a registry-backed
/// dispatcher so the overall service-name reflects the health of every
/// registered handler in the dispatcher.
pub struct HealthAggregate {
    service:    Arc<HealthService>,
    dispatcher: Arc<dyn GrpcInbound>,
}

impl HealthAggregate {
    /// Bind a [`HealthService`] to a dispatcher whose `health_check` is
    /// the source of truth for the overall status.  Callers refresh
    /// the status by invoking [`HealthAggregate::refresh`].
    pub fn new(service: Arc<HealthService>, dispatcher: Arc<dyn GrpcInbound>) -> Self {
        Self { service, dispatcher }
    }

    /// Re-poll the dispatcher and update the overall service status.
    pub async fn refresh(&self) {
        let h = self.dispatcher.health_check().await;
        let status = match h {
            Ok(c) if c.healthy => ServingStatus::Serving,
            _ => ServingStatus::NotServing,
        };
        self.service.set_overall_status(status);
    }
}

// ── proto wire codecs ─────────────────────────────────────────────────────────

/// Decode a `grpc.health.v1.HealthCheckRequest` (single optional `string service = 1`).
///
/// The proto-encoded form for an absent string is an empty body; for a
/// non-empty service name it is `0x0a <varint-len> <utf8-bytes>`.  We
/// accept any bytes and fall back to the empty service when decoding
/// fails — that mirrors how Google's reference health server treats
/// malformed Check requests.
pub(crate) fn decode_health_check_request(body: &[u8]) -> Option<String> {
    if body.is_empty() {
        return Some(String::new());
    }
    if body[0] != 0x0a {
        // Tag 1, wire type 2 (length-delimited) — anything else is not
        // the field we know about.  Accept unknown fields as no service.
        return Some(String::new());
    }
    let mut idx = 1usize;
    let (len, consumed) = decode_varint(&body[idx..])?;
    idx += consumed;
    if idx + (len as usize) > body.len() {
        return None;
    }
    std::str::from_utf8(&body[idx..idx + len as usize])
        .ok()
        .map(str::to_string)
}

/// Encode a `grpc.health.v1.HealthCheckResponse` (single `ServingStatus status = 1`).
///
/// Field 1 is an enum (varint).  When the value is 0 (UNKNOWN) the
/// proto3 default rule lets us emit an empty body; otherwise we emit
/// `0x08 <varint(value)>`.
pub(crate) fn encode_health_check_response(status: ServingStatus) -> Vec<u8> {
    let value = status as i32;
    if value == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(2);
    out.push(0x08); // tag 1, wire type 0 (varint)
    encode_varint(value as u64, &mut out);
    out
}

fn decode_varint(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut result = 0u64;
    let mut shift  = 0u32;
    for (i, byte) in bytes.iter().take(10).enumerate() {
        result |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift += 7;
    }
    None
}

fn encode_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;

    fn fresh_service() -> Arc<HealthService> { Arc::new(HealthService::new()) }

    /// @covers: HealthService::new — overall service starts as SERVING.
    #[test]
    fn test_new_health_service_overall_status_starts_serving() {
        let svc = fresh_service();
        assert_eq!(svc.get_status(""), Some(ServingStatus::Serving));
    }

    /// @covers: set_status — registers a new service.
    #[test]
    fn test_set_status_registers_new_named_service() {
        let svc = fresh_service();
        svc.set_status("pkg.A", ServingStatus::Serving);
        assert_eq!(svc.get_status("pkg.A"), Some(ServingStatus::Serving));
    }

    /// @covers: set_status — updates existing service.
    #[test]
    fn test_set_status_updates_existing_service_status() {
        let svc = fresh_service();
        svc.set_status("pkg.A", ServingStatus::Serving);
        svc.set_status("pkg.A", ServingStatus::NotServing);
        assert_eq!(svc.get_status("pkg.A"), Some(ServingStatus::NotServing));
    }

    /// @covers: set_overall_status — updates the empty-name slot.
    #[test]
    fn test_set_overall_status_updates_empty_service_slot() {
        let svc = fresh_service();
        svc.set_overall_status(ServingStatus::NotServing);
        assert_eq!(svc.get_status(""), Some(ServingStatus::NotServing));
    }

    /// @covers: get_status — unknown service returns None.
    #[test]
    fn test_get_status_returns_none_for_unregistered_service() {
        let svc = fresh_service();
        assert!(svc.get_status("not.registered").is_none());
    }

    /// @covers: recompute_overall_status — all serving = SERVING.
    #[test]
    fn test_recompute_overall_status_all_serving_yields_serving() {
        let svc = fresh_service();
        svc.set_status("pkg.A", ServingStatus::Serving);
        svc.set_status("pkg.B", ServingStatus::Serving);
        svc.recompute_overall_status();
        assert_eq!(svc.get_status(""), Some(ServingStatus::Serving));
    }

    /// @covers: recompute_overall_status — any not-serving = NOT_SERVING.
    #[test]
    fn test_recompute_overall_status_one_not_serving_taints_overall() {
        let svc = fresh_service();
        svc.set_status("pkg.A", ServingStatus::Serving);
        svc.set_status("pkg.B", ServingStatus::NotServing);
        svc.recompute_overall_status();
        assert_eq!(svc.get_status(""), Some(ServingStatus::NotServing));
    }

    /// @covers: subscribe — receives subsequent set_status updates.
    #[tokio::test]
    async fn test_subscribe_receives_subsequent_status_changes() {
        let svc = fresh_service();
        let mut rx = svc.subscribe();
        svc.set_status("pkg.A", ServingStatus::NotServing);
        let (name, status) = rx.recv().await.expect("must receive update");
        assert_eq!(name, "pkg.A");
        assert_eq!(status, ServingStatus::NotServing);
    }

    /// @covers: handle_unary — Check returns SERVING for a registered service.
    #[tokio::test]
    async fn test_check_returns_serving_for_registered_service() {
        let svc = fresh_service();
        svc.set_status("pkg.A", ServingStatus::Serving);
        let body = build_check_request_body("pkg.A");
        let req = GrpcRequest::new(HEALTH_CHECK_METHOD, body, Duration::from_secs(1));
        let resp = svc.handle_unary(req).await.expect("Check must succeed");
        assert_eq!(resp.body, encode_health_check_response(ServingStatus::Serving));
    }

    /// @covers: handle_unary — Check returns NotFound for unregistered service.
    #[tokio::test]
    async fn test_check_returns_not_found_for_unregistered_service() {
        let svc = fresh_service();
        let body = build_check_request_body("missing");
        let req = GrpcRequest::new(HEALTH_CHECK_METHOD, body, Duration::from_secs(1));
        let err = svc.handle_unary(req).await.expect_err("must error");
        assert!(matches!(err, GrpcInboundError::NotFound(_)), "{err:?}");
    }

    /// @covers: handle_unary — empty service name returns overall status.
    #[tokio::test]
    async fn test_check_with_empty_service_returns_overall_status() {
        let svc = fresh_service();
        svc.set_overall_status(ServingStatus::NotServing);
        let req = GrpcRequest::new(HEALTH_CHECK_METHOD, vec![], Duration::from_secs(1));
        let resp = svc.handle_unary(req).await.expect("Check must succeed");
        assert_eq!(resp.body, encode_health_check_response(ServingStatus::NotServing));
    }

    /// @covers: handle_unary — Watch is rejected via the unary path.
    #[tokio::test]
    async fn test_watch_via_unary_path_returns_unimplemented() {
        let svc = fresh_service();
        let req = GrpcRequest::new(HEALTH_WATCH_METHOD, vec![], Duration::from_secs(1));
        let err = svc.handle_unary(req).await.expect_err("must error");
        assert!(
            matches!(err, GrpcInboundError::Status(GrpcStatusCode::Unimplemented, _)),
            "{err:?}"
        );
    }

    /// @covers: handle_stream Watch — emits initial snapshot then change events.
    #[tokio::test]
    async fn test_watch_emits_initial_status_then_subsequent_changes() {
        use futures::StreamExt;
        let svc = fresh_service();
        svc.set_status("pkg.A", ServingStatus::Serving);
        let body = build_check_request_body("pkg.A");
        let request_stream: GrpcMessageStream = Box::pin(futures::stream::once(
            futures::future::ready(Ok(body)),
        ));
        let (mut out_stream, _meta) = svc
            .handle_stream(
                HEALTH_WATCH_METHOD.to_string(),
                GrpcMetadata::default(),
                request_stream,
            )
            .await
            .expect("watch must start");
        let initial = out_stream.next().await.expect("initial frame").expect("ok");
        assert_eq!(initial, encode_health_check_response(ServingStatus::Serving));

        // Toggle the status — Watch must push the new value.
        svc.set_status("pkg.A", ServingStatus::NotServing);
        let next = out_stream.next().await.expect("next frame").expect("ok");
        assert_eq!(next, encode_health_check_response(ServingStatus::NotServing));
    }

    /// @covers: handle_stream Watch — unknown service returns SERVICE_UNKNOWN.
    #[tokio::test]
    async fn test_watch_for_unknown_service_emits_service_unknown() {
        use futures::StreamExt;
        let svc = fresh_service();
        let body = build_check_request_body("never.registered");
        let request_stream: GrpcMessageStream = Box::pin(futures::stream::once(
            futures::future::ready(Ok(body)),
        ));
        let (mut out_stream, _) = svc
            .handle_stream(
                HEALTH_WATCH_METHOD.to_string(),
                GrpcMetadata::default(),
                request_stream,
            )
            .await
            .expect("watch must start");
        let initial = out_stream.next().await.expect("initial frame").expect("ok");
        assert_eq!(initial, encode_health_check_response(ServingStatus::ServiceUnknown));
    }

    /// @covers: encode_health_check_response — UNKNOWN encodes as empty body (proto3 default).
    #[test]
    fn test_encode_health_check_response_unknown_yields_empty_body() {
        let body = encode_health_check_response(ServingStatus::Unknown);
        assert!(body.is_empty(), "proto3 default for enum=0 is an empty body");
    }

    /// @covers: encode_health_check_response — SERVING encodes as 0x08 0x01.
    #[test]
    fn test_encode_health_check_response_serving_yields_two_byte_payload() {
        let body = encode_health_check_response(ServingStatus::Serving);
        assert_eq!(body, vec![0x08, 0x01]);
    }

    /// @covers: encode_health_check_response — NOT_SERVING encodes as 0x08 0x02.
    #[test]
    fn test_encode_health_check_response_not_serving_yields_two_byte_payload() {
        let body = encode_health_check_response(ServingStatus::NotServing);
        assert_eq!(body, vec![0x08, 0x02]);
    }

    /// @covers: decode_health_check_request — empty body decodes to "".
    #[test]
    fn test_decode_health_check_request_empty_body_yields_empty_service_name() {
        assert_eq!(decode_health_check_request(&[]), Some(String::new()));
    }

    /// @covers: decode_health_check_request — non-empty service round-trips.
    #[test]
    fn test_decode_health_check_request_round_trips_simple_service_name() {
        let body = build_check_request_body("pkg.A");
        assert_eq!(decode_health_check_request(&body), Some("pkg.A".to_string()));
    }

    /// @covers: decode_health_check_request — body whose tag is unknown decodes to "".
    #[test]
    fn test_decode_health_check_request_unknown_tag_yields_empty_service_name() {
        // Tag 2 (unknown to us) wire-type 2, length 0 — should be ignored.
        let body = vec![0x12, 0x00];
        assert_eq!(decode_health_check_request(&body), Some(String::new()));
    }

    /// @covers: HealthAggregate::refresh — propagates dispatcher health to overall.
    #[tokio::test]
    async fn test_health_aggregate_refresh_pushes_dispatcher_health_to_overall() {
        struct AlwaysHealthy;
        impl GrpcInbound for AlwaysHealthy {
            fn handle_unary(&self, _: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
                Box::pin(async { Ok(GrpcResponse { body: vec![], metadata: GrpcMetadata::default() }) })
            }
            fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
                Box::pin(async { Ok(GrpcHealthCheck::healthy()) })
            }
        }
        let svc = fresh_service();
        let agg = HealthAggregate::new(svc.clone(), Arc::new(AlwaysHealthy));
        svc.set_overall_status(ServingStatus::NotServing);
        agg.refresh().await;
        assert_eq!(svc.get_status(""), Some(ServingStatus::Serving));
    }

    /// @covers: HealthAggregate::refresh — unhealthy dispatcher flips overall.
    #[tokio::test]
    async fn test_health_aggregate_refresh_marks_overall_not_serving_when_dispatcher_unhealthy() {
        struct AlwaysUnhealthy;
        impl GrpcInbound for AlwaysUnhealthy {
            fn handle_unary(&self, _: GrpcRequest) -> BoxFuture<'_, GrpcInboundResult<GrpcResponse>> {
                Box::pin(async { Ok(GrpcResponse { body: vec![], metadata: GrpcMetadata::default() }) })
            }
            fn health_check(&self) -> BoxFuture<'_, GrpcInboundResult<GrpcHealthCheck>> {
                Box::pin(async { Ok(GrpcHealthCheck::unhealthy("nope")) })
            }
        }
        let svc = fresh_service();
        let agg = HealthAggregate::new(svc.clone(), Arc::new(AlwaysUnhealthy));
        agg.refresh().await;
        assert_eq!(svc.get_status(""), Some(ServingStatus::NotServing));
    }

    /// @covers: handle_stream — non-Watch method falls through to handle_unary.
    #[tokio::test]
    async fn test_handle_stream_falls_back_to_handle_unary_for_non_watch_method() {
        use futures::StreamExt;
        let svc = fresh_service();
        let body = build_check_request_body("");
        let request_stream: GrpcMessageStream = Box::pin(futures::stream::once(
            futures::future::ready(Ok(body)),
        ));
        let (mut out_stream, _meta) = svc
            .handle_stream(
                HEALTH_CHECK_METHOD.to_string(),
                GrpcMetadata::default(),
                request_stream,
            )
            .await
            .expect("Check via stream must succeed");
        let frame = out_stream.next().await.expect("frame").expect("ok");
        assert_eq!(frame, encode_health_check_response(ServingStatus::Serving));
    }

    /// @covers: HealthService::health_check — always reports healthy.
    #[tokio::test]
    async fn test_health_check_always_reports_healthy() {
        let svc = fresh_service();
        let h = svc.health_check().await.expect("health check ok");
        assert!(h.healthy);
    }

    /// Hand-build a `HealthCheckRequest` with `service = name`.
    fn build_check_request_body(name: &str) -> Vec<u8> {
        let bytes = name.as_bytes();
        if bytes.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(2 + bytes.len());
        out.push(0x0a); // tag 1, wire type 2 (length-delimited)
        encode_varint(bytes.len() as u64, &mut out);
        out.extend_from_slice(bytes);
        out
    }

    /// @covers: encode_varint, decode_varint — round-trip for a range of values.
    #[test]
    fn test_varint_round_trip_for_typical_values() {
        for v in [0u64, 1, 0x7f, 0x80, 0x3fff, 0x4000, 1_234_567] {
            let mut buf = Vec::new();
            encode_varint(v, &mut buf);
            let (out, consumed) = decode_varint(&buf).unwrap();
            assert_eq!(out, v, "round-trip failed for {v}");
            assert_eq!(consumed, buf.len(), "consumed mismatch for {v}");
        }
    }
}
