#!/usr/bin/env bash
# Bootstrap the HTTP middleware workspace — installs toolchain and dev deps.
set -euo pipefail

REQUIRED_RUST="1.75"

if ! command -v rustup &>/dev/null; then
    echo "Error: rustup not found. Install from https://rustup.rs/" >&2
    exit 1
fi

rustup toolchain install "${REQUIRED_RUST}" --component clippy,rustfmt
rustup override set "${REQUIRED_RUST}"
cargo fetch
echo "Bootstrap complete. Run 'cargo test' to verify."
