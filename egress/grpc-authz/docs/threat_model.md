# Threat Model — swe-edge-egress-grpc-authz

STRIDE analysis for the authz interceptor.

## Assets

- The authorisation decision (`allow` / `deny`) for a method+caller pair
- The verified caller identity carried in `GrpcMetadata`
- The configured policy (built-in `MethodAclConfig` or a consumer impl)

## Trust boundaries

- **Upstream** (REQUIRED): an authn interceptor (mTLS, bearer) has
  validated the caller and populated `GrpcMetadata` with verified
  identity keys.  The authz interceptor TRUSTS those keys.
- **Downstream**: the handler trusts the allow decision.

## Threats

### S — Spoofing

| Attack                                                          | Mitigation                                                                                                                                                  |
|----------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Caller injects `x-edge-peer-cn` over the wire                   | The ingress `TonicGrpcServer` strips reserved `x-edge-peer-*` keys at conn-accept time before any interceptor runs (see ingress dispatch).                   |
| Caller injects `x-edge-extracted-bearer-subject`                | The bearer interceptor (`swe-edge-egress-grpc-auth-bearer`) ALWAYS strips that key before validating, then re-inserts only on success.                       |
| Authz interceptor wired in front of authn                       | Fail-closed: `before_dispatch` returns `Unauthenticated` when no identity is present.  Tests cover this gate.                                                |

### T — Tampering

| Attack                                | Mitigation                                                                                                  |
|--------------------------------------|-------------------------------------------------------------------------------------------------------------|
| Mid-pipeline interceptor mutates CN  | Authz decision is taken inside its own `before_dispatch`; no later interceptor can re-run it on the same call. |
| TOML config tampered                 | Out of scope — config integrity is the deployment platform's responsibility.                                 |

### R — Repudiation

The interceptor logs every denial at WARN with the CN and method
path via `tracing`.  Consumers should forward this stream into
their access log so denied calls carry attribution.

### I — Information disclosure

| Attack                              | Mitigation                                                                                                                  |
|-------------------------------------|-----------------------------------------------------------------------------------------------------------------------------|
| Error message leaks policy details | `PermissionDenied` carries the static string `"authorization denied"` — no method name, no CN, no policy reason.            |
| Error message leaks identity         | Same — the `Unauthenticated` variant says `"no verified identity for authz"` without naming any header.                    |

### D — Denial of service

| Attack                                  | Mitigation                                                                                                  |
|----------------------------------------|-------------------------------------------------------------------------------------------------------------|
| Many calls force expensive policy eval | The built-in `MethodAclPolicy` is O(allowlist size); custom policies are the implementor's responsibility.   |
| Authz call stalls (e.g. external DB)   | The trait signature is sync — implementors that need async lookups must pre-cache.  Documented in trait docs. |

### E — Elevation of privilege

| Attack                                                          | Mitigation                                                                                                                                              |
|----------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------|
| Wildcard subject `"*"` mis-configured to permit dangerous methods | Empty by default in `config/application.toml`; consumers must explicitly opt in.  README documents the wildcard semantics.                              |
| Subject not present is treated as `*`                            | Explicitly tested: `MethodAclConfig::allows(None, …)` returns `false`.  Wildcard ONLY applies to authenticated callers (subject `Some(_)`).            |
| `MethodAclConfig` empty method list misread as "deny all"        | The `[]` value is a sentinel for "any method".  Documented in config schema, in API doc, and exercised in unit tests to lock the semantics in place.    |

## Extension points

| Need                                  | Approach                                                                                                              |
|--------------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| Async policy (DB / OPA / Cedar)       | Implement an internal cache and expose a sync `AuthzPolicy` over it.                                                  |
| Identity beyond CN                    | Implement `AuthzPolicy` directly and read from `PeerIdentity::san` / `custom_oids`; the metadata adapter is pub(crate). |
| Per-method quota                      | Out of scope — wrap the authz interceptor with a separate quota interceptor.                                          |
