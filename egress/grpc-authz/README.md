# swe-edge-egress-grpc-authz

Pluggable authorization `GrpcInboundInterceptor`.

## Wiring order

```
[ TLS-required server ] → mTLS / bearer interceptor → AuthzInterceptor → handler
                            (writes identity)         (reads identity)
```

The authz interceptor MUST run AFTER an authn interceptor — it
fail-closes with `Unauthenticated` when no identity is present in
the request metadata.

## Reading identity

`AuthzInterceptor::identity_from_metadata` consults two reserved
keys in priority order:

1. `x-edge-peer-cn` — set by the ingress server after a successful
   mTLS handshake.
2. `x-edge-extracted-bearer-subject` — set by
   `swe-edge-egress-grpc-auth-bearer` after a successful JWT
   verification.

The mTLS CN takes precedence when both are present.

## Custom policies

Implement `AuthzPolicy`:

```rust,ignore
use swe_edge_egress_grpc_authz::AuthzPolicy;
use swe_edge_ingress_grpc::PeerIdentity;

struct AlwaysAllow;
impl AuthzPolicy for AlwaysAllow {
    fn allows(&self, _id: &PeerIdentity, _method: &str) -> bool { true }
}
```

Closures `Fn(&PeerIdentity, &str) -> bool + Send + Sync` also implement
the trait, so the smallest possible test/dev wiring is:

```rust,ignore
use std::sync::Arc;
use swe_edge_egress_grpc_authz::AuthzInterceptor;
use swe_edge_ingress_grpc::GrpcInboundInterceptorChain;

let interceptor = AuthzInterceptor::from_policy(
    |_id: &swe_edge_ingress_grpc::PeerIdentity, method: &str| method.starts_with("/svc/Read"),
);
let chain = GrpcInboundInterceptorChain::new().push(Arc::new(interceptor));
```

## Built-in policy: method-ACL

`MethodAclPolicy` consults a config-driven allowlist keyed on the
caller's CN.  See `config/application.toml` for the schema and
`docs/threat_model.md` for security trade-offs.

## Threat model

See `docs/threat_model.md`.
