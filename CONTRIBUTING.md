# Contributing to swe-edge

Thank you for your interest in contributing to swe-edge.

## Getting Started

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run tests and linting (see below)
5. Commit your changes
6. Push to the branch (`git push origin feature/my-feature`)
7. Open a Pull Request

## Development Setup

swe-edge has five independent Rust workspaces. There is no root `Cargo.toml` — build and test each workspace separately:

```bash
cd ingress && cargo build && cargo test && cargo clippy -- -D warnings
cd egress  && cargo build && cargo test && cargo clippy -- -D warnings
cd proxy   && cargo build && cargo test && cargo clippy -- -D warnings
cd domain  && cargo build && cargo test && cargo clippy -- -D warnings
cd runtime && cargo build && cargo test && cargo clippy -- -D warnings
```

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy -- -D warnings` — zero warnings is the bar
- All public items must have documentation comments (`#![warn(missing_docs)]` is enforced)
- No `unsafe` code (`#![deny(unsafe_code)]` is enforced)

## SEA Module Layout

Every crate in this repo follows the SEA (Structural Engineering Architecture) module contract:

```
src/
├── api/    # Public traits and value objects only
├── core/   # Implementations — pub(crate), never exposed directly
├── saf/    # Sole public factory/facade surface
└── lib.rs  # Re-exports via pub use saf::*
```

Consumers call `saf/` factories that return `impl Trait`. Concrete types in `core/` are never part of the public API.

## Testing Conventions

- Integration tests: `tests/*_int_test.rs` or `tests/*_e2e_test.rs`
- Unit tests: inline `#[cfg(test)]` modules
- All async tests use `#[tokio::test]`
- Test naming: `test_<action>_<condition>_<expectation>`

## Reporting Issues

Please use the [GitHub issue tracker](https://github.com/sweengineeringlabs/edge/issues) to report bugs or request features.
