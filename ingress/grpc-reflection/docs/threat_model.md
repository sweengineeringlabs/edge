# Threat Model — swe-edge-ingress-grpc-reflection

STRIDE analysis for the reflection RPC.

## Assets

- The list of registered service/method paths (information asset).
- The FileDescriptorProto bytes for each registered descriptor
  (information asset — may include comments, internal naming,
  obsolete fields).
- The configuration flag `GrpcServerConfig::enable_reflection`
  (control asset — flipping it from `false` to `true` exposes
  every information asset above).

## Trust boundaries

- The reflection RPC is **always reachable** when registered — it
  has no per-call authorisation hook of its own.
- The Phase 3 server-level authorisation invariant
  (`AuthorizationInterceptor` required, or
  `allow_unauthenticated = true` opt-in) still applies — the
  reflection method path goes through the same chain as every
  other handler.

## Threats

### S — Spoofing

| Attack | Mitigation |
|---|---|
| Caller forges a `ServerReflectionRequest` to enumerate methods. | Reflection is informational only — the worst the caller can do is learn the service surface. The Phase 3 authz invariant gates whether they can reach the endpoint at all. |

### T — Tampering

| Attack | Mitigation |
|---|---|
| Caller sends a malformed `ServerReflectionRequest`. | The hand-rolled codec returns `ReflectionError::Malformed`, surfaced as a structured `ErrorResponse(INVALID_ARGUMENT)` rather than panicking. |
| Truncated string field. | Length-checked decode returns `Malformed`. |
| Unknown wire type. | `skip_field` rejects wire types 6/7 (group start/end) loudly. |

### R — Repudiation

The Phase 3 audit sink fires for every reflection call exactly as
it does for normal handlers — the method path
`/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo`
appears in the audit log.

### I — Information disclosure

| Attack | Mitigation |
|---|---|
| Attacker enumerates the service surface in production. | **Default-off**: `GrpcServerConfig::enable_reflection = false`. The server logs a WARN at startup when an operator opts in, so the decision is observable in deployment logs. |
| Descriptor blobs leak internal-only proto fields. | Operators control which `Descriptor`s are registered — only the bytes passed to `with_descriptor_set` are served. The crate never auto-discovers descriptors from disk. |
| `original_request` echoed in response leaks identity-bearing host string. | Reflection responses pre-existed before our impl; `host` is purely advisory and is present in every spec-compliant server's responses. |

### D — Denial of service

| Attack | Mitigation |
|---|---|
| Attacker sends a giant `ServerReflectionRequest`. | The Phase 2 `max_message_bytes` cap (default 4 MiB) applies. |
| Attacker opens many bidi streams. | The Phase 2 `max_concurrent_streams` cap applies. |
| Attacker drives `ListServices` repeatedly. | `list_services` is O(N) over the registry; the registry is a `parking_lot::RwLock<HashMap>` — read-locks are non-exclusive, so concurrent `ListServices` calls don't block one another. |
| Slow `Watch`-style consumer (n/a here — we don't expose Watch). | Not applicable. |

### E — Elevation of privilege

| Attack | Mitigation |
|---|---|
| Caller exploits reflection to bypass authz on other methods. | Reflection only describes services; it never invokes them. The dispatcher's own authz chain still gates every other RPC. |
| Caller registers their own descriptor. | The reflection service has no public mutation API once handed to the dispatcher — `add_descriptor` and `with_descriptors` are consumed by the operator at startup. |

## Operational guidance

- **Production**: leave `enable_reflection = false`. If grpcurl
  access is required for ops, use a control-plane network gate
  (mTLS-only ACL on a separate listener, port-knocking, or a
  short-lived feature flag) rather than enabling the flag on the
  public endpoint.
- **Staging / dev**: opt in via
  `GrpcServerConfig::enable_reflection()`; the WARN log makes the
  decision auditable.
- **Descriptors**: only register the FileDescriptorProto bytes for
  protos a debugging operator legitimately needs to inspect. The
  bytes are served verbatim — strip internal-only options upstream
  if your protos carry comments or naming you treat as sensitive.
