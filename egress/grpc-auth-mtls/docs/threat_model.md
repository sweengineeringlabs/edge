# Threat Model — swe-edge-egress-grpc-auth-mtls

STRIDE analysis for the mTLS interceptor.

## Assets

- The `x-edge-peer-cert-fingerprint-sha256` metadata key (proof of
  successful mTLS handshake)
- The CN/SAN attributes the policy authorises against
- Downstream handler invocation rights

## Trust boundaries

- **Upstream**: the ingress `TonicGrpcServer` performs the actual
  TLS/mTLS handshake.  This interceptor TRUSTS that the server only
  injects `x-edge-peer-*` keys after a verified client cert.
- **Downstream**: the handler trusts whatever this interceptor
  passes through.

## Threats

### S — Spoofing

| Attack                                                     | Mitigation                                                                                                         |
|-----------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------|
| Client sets `x-edge-peer-cn: admin` over the wire         | The ingress `TonicGrpcServer` strips all `x-edge-peer-*` headers from incoming metadata before injecting its own values (`is_reserved_peer_key` filter in `dispatch`).  This interceptor never observes spoofed values. |
| Plaintext / TLS-only client tries to look authenticated   | No fingerprint key is set on non-mTLS conns — interceptor returns `Unauthenticated`.                               |
| Verified peer with CN matching a wildcard                 | Only exact case-insensitive CN matching is supported (no wildcards / globs).                                       |

### T — Tampering

| Attack                                       | Mitigation                                                                                          |
|---------------------------------------------|-----------------------------------------------------------------------------------------------------|
| In-flight modification of the cert chain    | Out of scope — TLS itself protects integrity.                                                       |
| Mid-pipeline interceptor mutates `x-edge-peer-*` | Interceptor reads its values at `before_dispatch` time; later interceptors mutating them affect only the handler view, not the auth decision. |

### R — Repudiation

The fingerprint is logged at the server level (TonicGrpcServer
trace logs) so that a denied call carries enough audit evidence
to attribute it to a specific peer cert.  Consumers should
forward fingerprints into their access log.

### I — Information disclosure

| Attack                                | Mitigation                                                                                       |
|--------------------------------------|--------------------------------------------------------------------------------------------------|
| Error messages leak peer CN to caller| `PermissionDenied` carries a generic `"peer identity is not on the allowlist"` — the CN is not echoed. |
| TOML config leaks PII via paths      | Config holds CNs/SANs only — no credentials, no paths.                                           |

### D — Denial of service

| Attack                              | Mitigation                                                              |
|-------------------------------------|-------------------------------------------------------------------------|
| Many small connections to exhaust  | Interceptor work is O(allowlist size); the server's TLS / connection-bound caps apply upstream. |

### E — Elevation of privilege

| Attack                                                | Mitigation                                                                                                         |
|------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------|
| Method bypass list configured too broadly             | `allow_unauthenticated_methods` is `false` by default; consumers must explicitly opt in and list each method path. |
| CN allowlist is empty interpreted as "permit all"    | True by design — empty == "any verified peer".  Documented in README; CI tests verify this default.                |
