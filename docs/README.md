# Edge

**Audience**: Developers, architects

Embeddable, library-level HTTP/gRPC dispatch stack for Rust services.

Edge enforces the SEA (Structural Engineering Architecture) module contract at the type level — no sidecar process, no framework lock-in. Consumer code imports only traits from `api/` and calls factories from `saf/`. The transport is always behind the trait and swappable without touching business logic.

## Workspaces

| Workspace | Crate prefix | Role |
|-----------|-------------|------|
| `ingress/` | `swe-edge-ingress-*` | Inbound ports — `HttpInbound`, `GrpcInbound`, `FileInbound` |
| `egress/` | `swe-edge-egress-*` | Outbound ports + 7 middleware crates |
| `proxy/` | `edge-proxy` | Dispatch — `Job → Router → LifecycleMonitor` |
| `domain/` | `edge-domain` | Business logic — `Handler → HandlerRegistry` |
| `runtime/` | `swe-edge-runtime` | Wires all layers — `RuntimeManager`, graceful shutdown |

## Quick Links

- [Rationale](0-research/rationale.md) — why Edge exists
- [Architecture](3-architecture/architecture.md) — diagrams and layer model
- [Developer Guide](4-development/developer_guide.md) — build, extend, contribute
- [Testing Strategy](5-testing/testing_strategy.md) — test categories and conventions
- [Deployment Guide](6-operations/deployment_guide.md) — consuming the library in production

## Source

[github.com/sweengineeringlabs/edge](https://github.com/sweengineeringlabs/edge)
