# Edge Setup Guide

**Audience**: New contributors, developers setting up the repository for the first time

## Prerequisites

| Requirement | Minimum Version | Check Command |
|-------------|-----------------|---------------|
| Rust toolchain | 1.75 | `rustc --version` |
| Cargo | 1.75 | `cargo --version` |
| Git | 2.x | `git --version` |
| mdBook (docs only) | 0.4.40 | `mdbook --version` |
| mdbook-mermaid (docs only) | 0.14.0 | `mdbook-mermaid --version` |

## Repository Setup

### Clone and Navigate

```bash
git clone https://github.com/sweengineeringlabs/edge
cd edge
```

### Install Rust Toolchain

```bash
# Install rustup if not present
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Ensure stable toolchain is active
rustup default stable
rustup update stable
```

## Build

Edge has no root `Cargo.toml`. Build each workspace independently:

```bash
cd ingress && cargo build
cd ../egress  && cargo build
cd ../proxy   && cargo build
cd ../domain  && cargo build
cd ../runtime && cargo build
```

Build a single crate within a workspace:

```bash
cd egress
cargo build -p swe-edge-egress-auth
```

## Test

```bash
# All tests in a workspace
cd ingress && cargo test

# Unit tests only
cargo test --lib

# Single integration test file
cargo test --test axum_server_int_test

# Single test function with output
cargo test test_server_routes_get_request_to_handler_and_returns_200 -- --nocapture

# All tests in a specific crate
cargo test -p swe-edge-ingress-http
```

## Lint and Format

```bash
# Format check (CI bar)
cargo fmt --check

# Apply formatting
cargo fmt

# Lint (zero warnings required)
cargo clippy -- -D warnings
```

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `RUST_LOG` | Log verbosity filter | `info` |
| `SD_NOTIFY_SOCKET` | systemd notify socket path | unset |
| `MY_API_TOKEN` | Example: bearer token for egress auth middleware | — |

Secrets are always read from environment variables at runtime. Never commit token values.

## IDE Setup

### VS Code

Recommended extensions:
- `rust-analyzer` — language server, inline type hints
- `Even Better TOML` — Cargo.toml and config file support
- `CodeLLDB` — native debugging

### IntelliJ / RustRover

- Install the Rust plugin (or use RustRover directly)
- Set the project toolchain to stable
- Each workspace directory (`ingress/`, `egress/`, etc.) is an independent Cargo project — open them separately or attach all five as modules

## Building the Documentation Book

```bash
# Install tooling (one-time)
cargo install mdbook --version 0.4.40
cargo install mdbook-mermaid --version 0.14.0

# Inject mermaid support into book.toml (one-time per clone)
mdbook-mermaid install .

# Build the book
mdbook build

# Serve locally with live reload
mdbook serve --open
```

The built book is written to `book/` (gitignored). The live site is at https://sweengineeringlabs.github.io/edge/.

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `cargo build` fails with missing dependency | Run `cargo fetch` then retry |
| Tests hang or timeout | Check for blocking I/O in async tests; all async tests must use `#[tokio::test]` |
| Clippy warnings on CI but not locally | Ensure your local Rust version matches CI (`rustup update stable`) |
| TLS tests fail with `no default provider` | Call `rustls::crypto::aws_lc_rs::default_provider().install_default().ok()` at the top of the test |
| `mdbook serve` fails with mermaid rendering | Re-run `mdbook-mermaid install .` to regenerate `mermaid.min.js` and `mermaid-init.js` |

## See Also

- [Developer Guide](developer_guide.md)
- [Architecture](../3-architecture/architecture.md)
- [Overview](../README.md)
