# swe-edge-egress-grpc-auth-bearer

Symmetric JWT bearer interceptors for the gRPC stack.

Two implementors ship together so a typical service-to-service hop
can wire the same crate on both ends:

- `BearerOutboundInterceptor` (implements
  `swe_edge_egress_grpc::GrpcOutboundInterceptor`) — mints a JWT
  from configured claims and attaches `authorization: Bearer <jwt>`
  to every outbound request.
- `BearerInboundInterceptor` (implements
  `swe_edge_ingress_grpc::GrpcInboundInterceptor`) — validates the
  bearer token, then republishes the verified `sub` claim under
  the internal metadata key `x-edge-extracted-bearer-subject` for
  downstream authz policies.

Algorithms: HS256 (raw bytes) and RS256 (PEM keys).

## Wire-shape contract

| Direction | Header           | Format                    |
|-----------|------------------|---------------------------|
| Outbound  | `authorization` | `Bearer <jwt>`            |
| Inbound   | `authorization` | `Bearer <jwt>` (read)     |
| Inbound   | `x-edge-extracted-bearer-subject` | written after success — never trusted from the wire |

The inbound interceptor ALWAYS strips any incoming
`x-edge-extracted-bearer-subject` value before validating, so
downstream code can treat the post-interceptor key as authoritative.

## Constant-time secret comparison

`BearerSecret::ct_eq_hs256` uses `subtle::ConstantTimeEq` for raw
secret material.  Token *content* comparisons are delegated to
`jsonwebtoken`, which uses constant-time MAC verification internally.

## Quick start

### Outbound (client)

```rust,ignore
use std::sync::Arc;
use swe_edge_egress_grpc::{GrpcOutboundInterceptorChain, TonicGrpcClient};
use swe_edge_egress_grpc_auth_bearer::{
    BearerOutboundConfig, BearerOutboundInterceptor, BearerSecret,
};

let cfg = BearerOutboundConfig {
    secret: BearerSecret::Hs256 { secret: secret_bytes },
    issuer: "svc-a".into(),
    audience: "svc-b".into(),
    subject: "svc-a-client".into(),
    lifetime_seconds: 60,
};
let chain = GrpcOutboundInterceptorChain::new()
    .push(Arc::new(BearerOutboundInterceptor::from_config(cfg)));
let client = TonicGrpcClient::new(endpoint).with_interceptors(chain);
```

### Inbound (server)

```rust,ignore
use std::sync::Arc;
use swe_edge_ingress_grpc::{GrpcInboundInterceptorChain, TonicGrpcServer};
use swe_edge_egress_grpc_auth_bearer::{
    BearerInboundConfig, BearerInboundInterceptor, BearerSecret,
};

let cfg = BearerInboundConfig {
    secret: BearerSecret::Hs256 { secret: secret_bytes },
    expected_issuer: "svc-a".into(),
    expected_audience: "svc-b".into(),
    leeway_seconds: 5,
};
let chain = GrpcInboundInterceptorChain::new()
    .push(Arc::new(BearerInboundInterceptor::from_config(cfg)));
let server = TonicGrpcServer::new(bind, handler).with_interceptors(chain);
```

## Threat model

See `docs/threat_model.md`.
