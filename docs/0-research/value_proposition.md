# Edge — Market Research & Rationale

## What exists

| Tool | Type | Dispatch model | Language |
|------|------|----------------|----------|
| Envoy | Infrastructure proxy (sidecar) | L4/L7 proxy | C++ |
| Nginx / OpenResty | Infrastructure proxy | Reverse proxy | C / Lua |
| Traefik | Infrastructure proxy | Ingress controller | Go |
| Linkerd / Istio | Service mesh | Sidecar + control plane | Go / Rust |
| Tower (tokio-rs) | Rust middleware library | `Service` trait stack | Rust |
| Axum | Rust HTTP framework | Router + handler | Rust |
| Tonic | Rust gRPC framework | Server + client | Rust |

## Why none of them fit

### Infrastructure proxies (Envoy, Nginx, Traefik, Linkerd)

These run as separate processes. Your service binary calls out to a sidecar over a loopback socket. That adds latency, a second process to deploy, a second process to monitor, and a second failure domain. For library-first Rust services — especially embedded, CLI, or resource-constrained deployments — a sidecar is not an option.

### Tower / Axum / Tonic

Tower gives you a composable `Service` abstraction but no architectural contract on how layers are organised, named, or separated. Axum and Tonic are excellent HTTP and gRPC frameworks but they expose their own concrete types (routers, extractors, codegen stubs) directly to application code. There is no enforced boundary between transport, dispatch, and business logic.

The result: applications import Axum types into domain code, Tonic-generated stubs into business logic, and end up with framework-coupled application code that is hard to test without spinning up a real server.

## What Edge provides

Edge is an **embeddable, library-level dispatch stack** that enforces the SEA (Structural Engineering Architecture) module contract at the type level:

- **Ingress ports** (`swe-edge-ingress`) — `HttpInbound`, `GrpcInbound`, `FileInbound` traits with no framework types leaking into the API surface. Concrete servers (Axum, Tonic) live in `core/` behind `saf/` factories.
- **Egress ports** (`swe-edge-egress`) — `HttpOutbound`, `GrpcOutbound`, `DatabaseGateway`, `NotificationSender`, `PaymentGateway` traits. Middleware (auth, retry, rate, breaker, cache, TLS) composes via `reqwest-middleware` and is assembled by `DefaultHttpOutbound`.
- **Dispatch** (`edge-proxy`, `edge-domain`) — `Job → Router → HandlerRegistry → Handler` pipeline with typed errors and a `LifecycleMonitor` contract.
- **Runtime** (`swe-edge-runtime`) — wires all layers into a single `RuntimeManager` with graceful shutdown and systemd-notify support.

The key difference: application code imports only traits from `api/` and calls factories from `saf/`. No Axum, Tonic, or reqwest types cross the boundary. The transport is swappable without touching business logic.

## Who this is for

Open-source Rust services that want production-grade HTTP/gRPC dispatch without adopting a full framework or a sidecar mesh. The target is teams who write Rust for backends, embedded systems, or CLI tooling and want architectural guardrails rather than ad-hoc middleware stacks.
