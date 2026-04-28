# swe-edge-egress-grpc-auth-mtls

`GrpcInboundInterceptor` that **enforces** the presence of an mTLS
peer identity in the request metadata, optionally restricted to a
CN/SAN allowlist.

## Wire-shape contract

This crate consumes the metadata keys that the ingress
`TonicGrpcServer` injects after a successful client-cert handshake:

| Key                                  | Meaning                                    |
|--------------------------------------|--------------------------------------------|
| `x-edge-peer-cert-fingerprint-sha256`| **Required** — proof of mTLS handshake     |
| `x-edge-peer-cn`                     | Subject CN (when present)                  |
| `x-edge-peer-san-dns`                | Comma-separated DNS SANs (when present)    |

Only the SHA-256 fingerprint is treated as ground-truth identity
evidence — CN and SAN are absent on degenerate certs and absence is
not interpreted as "no mTLS".

## Decision rules

1. If `allow_unauthenticated_methods = true` and the request method
   path is in `unauthenticated_methods`, the call passes (used for
   health-check probes from load balancers that don't carry a client
   cert).
2. If no fingerprint is present, return `Unauthenticated`.
3. If `allowed_cns` is non-empty and the peer's CN doesn't match
   (case-insensitive), return `PermissionDenied`.
4. If `allowed_san_dns` is non-empty and **none** of the peer's DNS
   SANs match (case-insensitive), return `PermissionDenied`.
5. Otherwise, accept.

## Quick start

```rust,ignore
use std::sync::Arc;
use swe_edge_egress_grpc_auth_mtls::{MtlsAuthConfig, MtlsAuthInterceptor};
use swe_edge_ingress_grpc::{GrpcInboundInterceptorChain, TonicGrpcServer};

let auth = MtlsAuthInterceptor::from_config(
    MtlsAuthConfig::restrict_to_cns(["svc-trusted".into()])
);
let chain = GrpcInboundInterceptorChain::new().push(Arc::new(auth));
let server = TonicGrpcServer::new(bind, handler)
    .with_tls(tls_cfg) // mTLS-required upstream
    .with_interceptors(chain);
```

## Threat model

See `docs/threat_model.md` for the STRIDE breakdown.
