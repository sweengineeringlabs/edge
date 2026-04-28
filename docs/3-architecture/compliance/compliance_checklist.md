# Edge Compliance Checklist

**Audience**: Developers, architects, code reviewers

Use this checklist during code review for any PR that touches Edge's architectural boundaries. Every item must pass before merge. This checklist enforces the architecture defined in [`docs/3-architecture/architecture.md`](../architecture.md).

---

## How to Use

1. Run through this checklist on every PR that touches `api/`, `core/`, or `saf/` in any workspace
2. Automated items: run the commands and paste output into the review comment
3. Manual items: reviewer must explicitly confirm before approving

---

## 1. SEA Interface Compliance

Reference: `docs/3-architecture/architecture.md` — SEA Module Layout

### 1.1 Dependency Direction

- [ ] No consumer code imports from `core/` directly — all access through `saf/` or `api/`
- [ ] `core/` types are `pub(crate)` — no `pub struct` in `core/` that is not re-exported via `saf/`
- [ ] `api/` has no dependencies on `core/` — leaf layer only
- [ ] `saf/` factories return `impl Trait` — no concrete type named in the return position

**Verify**:
```bash
# Confirm no pub struct leaks from core/ into api/
grep -rn "^pub struct" ingress/*/src/core/ egress/*/src/core/ runtime/*/src/core/
# Each result must be pub(crate) or exposed only via saf/

# Build catches direction violations
cd ingress && cargo build
cd egress  && cargo build
cd proxy   && cargo build
cd domain  && cargo build
cd runtime && cargo build
```

### 1.2 Interface Stability

- [ ] Public API changes are intentional — trait method additions are breaking changes and require a semver bump
- [ ] No `axum`, `tonic`, or `reqwest` types appear in `api/` or `saf/` public signatures
- [ ] Value objects in `api/` are owned types (`String`, `Vec<u8>`) — no references or lifetime parameters in trait signatures

**Verify**:
```bash
# No framework types in public surface
grep -rn "axum::\|tonic::\|reqwest::" \
  ingress/*/src/api/ ingress/*/src/saf/ \
  egress/*/src/api/  egress/*/src/saf/  \
  runtime/*/src/api/ runtime/*/src/saf/
# Expected: no output
```

---

## 2. Middleware Compliance

Reference: `docs/3-architecture/architecture.md` — HTTP Middleware Pipeline

### 2.1 Policy in TOML

- [ ] No middleware policy is hardcoded as Rust literals — all config values come from TOML or env vars
- [ ] Each middleware crate has a `config/default.toml` with documented defaults
- [ ] Config struct derives `serde::Deserialize`; no `impl Default` that buries defaults in Rust

**Verify**:
```bash
# No hardcoded numeric policy in middleware src/
grep -rn "max_attempts\s*=\s*[0-9]\|requests_per_second\s*=\s*[0-9]\|failure_threshold\s*=\s*[0-9]" \
  egress/auth/src/ egress/retry/src/ egress/rate/src/ egress/breaker/src/ egress/cache/src/
# Expected: no output (values must come from config)
```

### 2.2 Middleware Independence

- [ ] Each middleware crate compiles independently — no circular dependencies between middleware crates
- [ ] New middleware added to `DefaultHttpOutbound` in `egress/http/src/core/outbound.rs`
- [ ] New middleware has its own `config/default.toml` entry and documented TOML section

---

## 3. Testing Compliance

Reference: `docs/5-testing/testing_strategy.md`

### 3.1 Integration Tests Required

- [ ] Every new port trait implementation has at least one integration test (`tests/*_int_test.rs`)
- [ ] Integration test covers: happy path, at least one error path, at least one boundary condition
- [ ] Integration tests bind on port 0 — no hardcoded port numbers

**Verify**:
```bash
# Check for hardcoded ports in test files
grep -rn "127\.0\.0\.1:[0-9]\{4,\}\|0\.0\.0\.0:[0-9]\{4,\}" \
  ingress/*/tests/ egress/*/tests/ runtime/*/tests/
# Expected: no output
```

### 3.2 Test Naming

- [ ] All test functions follow `test_<action>_<condition>_<expectation>`
- [ ] No test named `test_it_works`, `test_basic`, or `test_<type_name>` without further qualification

---

## 4. Dependency Compliance

### 4.1 No Unsafe Code

- [ ] No `unsafe` blocks anywhere in the codebase

**Verify**:
```bash
grep -rn "unsafe" ingress/*/src/ egress/*/src/ proxy/src/ domain/src/ runtime/*/src/
# Expected: no output
```

### 4.2 Workspace Dependency Alignment

- [ ] Shared dependencies (tokio, serde, thiserror) use consistent versions across workspaces
- [ ] No workspace pulls in a conflicting major version of a shared dependency

**Verify**:
```bash
cd ingress && cargo tree --duplicates
cd egress  && cargo tree --duplicates
cd runtime && cargo tree --duplicates
# Review duplicates — minor version differences are acceptable; major version splits are not
```

---

## 5. Security Compliance

- [ ] No secrets, tokens, or credentials in source files or committed config files
- [ ] Error responses do not leak internal paths, stack traces, or dependency versions
- [ ] Input validation occurs at the transport boundary — `HttpRequest` / `GrpcRequest` constructors validate size limits

---

## Quick Summary Table

Copy into your review comment:

| Category | Checks | Pass | Fail |
|----------|--------|------|------|
| SEA interface compliance | 1.1–1.2 | | |
| Middleware compliance | 2.1–2.2 | | |
| Testing compliance | 3.1–3.2 | | |
| Dependency compliance | 4.1–4.2 | | |
| Security compliance | 5 | | |
| **Total** | | | |

---

## Automated Check Script

```bash
#!/usr/bin/env bash
set -e

echo "=== SEA: no framework types in public API ==="
grep -rn "axum::\|tonic::\|reqwest::" \
  ingress/*/src/api/ ingress/*/src/saf/ \
  egress/*/src/api/  egress/*/src/saf/  \
  runtime/*/src/api/ runtime/*/src/saf/ || echo "PASS"

echo "=== Safety: no unsafe code ==="
grep -rn "unsafe" ingress/*/src/ egress/*/src/ proxy/src/ domain/src/ runtime/*/src/ || echo "PASS"

echo "=== Tests: no hardcoded ports ==="
grep -rn "127\.0\.0\.1:[0-9]\{4,\}\|0\.0\.0\.0:[0-9]\{4,\}" \
  ingress/*/tests/ egress/*/tests/ runtime/*/tests/ || echo "PASS"

echo "=== Build ==="
for ws in ingress egress proxy domain runtime; do
  (cd $ws && cargo build 2>&1 | tail -1) && echo "$ws: PASS"
done

echo "=== Clippy ==="
for ws in ingress egress proxy domain runtime; do
  (cd $ws && cargo clippy -- -D warnings 2>&1 | tail -1) && echo "$ws: PASS"
done
```
