# Bootstrap the HTTP middleware workspace — installs toolchain and dev deps.
#Requires -Version 7

$RequiredRust = "1.75"

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Error "rustup not found. Install from https://rustup.rs/"
    exit 1
}

rustup toolchain install $RequiredRust --component clippy,rustfmt
rustup override set $RequiredRust
cargo fetch
Write-Host "Bootstrap complete. Run 'cargo test' to verify."
