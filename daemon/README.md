# swe-edge-daemon

Process-level runtime manager for `swe-edge`. Wires ingress, controller, and egress into a single lifecycle-managed process with optional systemd `sd_notify` integration.

## Build

```bash
cargo build
```

## Test

```bash
cargo test --workspace
```

## Project Structure

- `main/src/api/` — Public traits and types (L2)
- `main/src/core/` — Implementation layer (L3)
- `main/src/gateway/` — Ingress/egress boundary types
- `main/src/saf/` — Public facade: `runtime_manager()` factory (L4)
