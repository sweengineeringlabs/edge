# Crate Naming Convention

**Scope.** Applies to every Rust crate published under the `swe-*` brand
across all workspaces (llmboot, vmisolate, security, swedge, …).

## Rule

A crate's package name is built by walking the directory tree from the
umbrella workspace down to the crate root, prefixed by the umbrella name:

```
<umbrella>_<domain>_<module>_<submodule>
```

The `<umbrella>` token carries `<brand>` + `<project>`, either joined by
an underscore (`swe_observ`) or contracted into a portmanteau (`swedge`
= swe + edge). Contraction is preferred when the contracted form reads
naturally and is unambiguous; otherwise use the underscore form.

**Each remaining segment corresponds to a real directory level.** You
never invent a slot to meet a fixed count, and you never duplicate a
name to pad to a "standard" length.

- In `Cargo.toml`, use the **kebab-case** form:
  `name = "swedge-gateway"`. This matches Rust ecosystem convention
  (`tokio-util`, `async-trait`, `serde-json`) and satisfies the
  struct-engine audit.
- In module paths, Cargo auto-converts hyphens to underscores:
  `use swedge_gateway::…`.
- Throughout this document the structural name is written in the
  underscore form because that is what appears in code.

**Rule of thumb: kebab in the manifest, underscore in imports.** One
crate, two spellings — Cargo does the conversion.

## Slot meanings

| Slot | Role | Example |
|------|------|---------|
| `<umbrella>` | Brand + project fused into one token (portmanteau) or joined by `_` | `swedge` (= swe + edge), `swe_observ`, `swe_iam` |
| `<domain>` | Functional area within the umbrella | `gateway`, `controlroom`, `context` |
| `<module>` | Sub-split of a domain (only if domain has multiple crates) | `contract`, `adapters`, `runtime` |
| `<submodule>` | Further nesting (rare) | — |

## Umbrella naming guidance

- Prefer the contracted form (`swedge`) when the join reads naturally and
  there is no ambiguity. It keeps crate names short.
- Fall back to the underscore form (`swe_observ`) when contraction would
  muddle the brand or obscure the project (e.g. `sweobserv` is harder to
  parse).
- Whichever form you choose, *keep it stable across the project's
  lifetime* — renaming it renames every downstream crate.

## Worked examples

```
swedge/                                       umbrella=swedge (swe + edge)
├── main/
│   └── features/
│       ├── gateway/{api,core,saf}      →  swedge_gateway        (2 parts)
│       └── controlroom/{api,core,saf}  →  swedge_controlroom    (2 parts)
```

If `gateway` later splits into `gateway/contract/` + `gateway/adapters/`,
names grow naturally to `swedge_gateway_contract` and
`swedge_gateway_adapters`. A slot materializes when a real directory
level materializes — never before.

Other umbrellas:

```
<llmboot>/main/features/backend/agents/runtime/pattern/react/
  → llmboot_react                             (contracted umbrella pattern applies)

<swe-observ>/main/backend/features/context/
  → swe_observ_context                        (underscored umbrella)
```

## Duplicates are a design signal, not a style issue

A repeated segment anywhere in the crate path means one of:

- **Forced split** — a module level that doesn't carry a real
  distinction. Collapse it.
- **Conceptual muddle** — two real levels genuinely claim the same label.
  The design has the same noun meaning two different things. Rename the
  level whose meaning is more specific than the project name.
- **Padding** — a name repeated to reach an artificial slot count.
  Remove the padding.

Don't rename around a duplicate. Trace it to the structural cause and
fix that.

| Name | What it signals | Fix |
|------|-----------------|-----|
| `swedge_gateway_edge` | `edge` appears as project and module | Collapse to `swe_edge_gateway` OR rename the module to what it specifically contains (`adapters`, `runtime`) |
| `swedge_controlroom_controlroom` | Module repeats its parent domain | Collapse to `swe_edge_controlroom` |
| `swe_edge_observ_observ_context` | Domain and module both `observ` | Collapse or rename the module to what it specifically is |

## Depth budget

| Parts | Status |
|-------|--------|
| 2 (`umbrella_domain`) | Normal — domain has one crate (contracted umbrella). |
| 3 (`umbrella_domain_module` or `brand_project_domain`) | Normal — domain has multiple crates, or the umbrella is uncontracted. |
| 4 (`umbrella_domain_module_submodule` or equivalent) | Allowed but rare. |
| 6+ | **Design review.** The tree is telling you something wants to be a separate workspace member at a higher level, or the hierarchy wants to flatten. |

Depth is a design signal. Don't route around it.

## Relationship to SEA internal layers

Inside each crate, the SEA layout (`src/{api,core,saf,spi,gateway}/`)
governs *module* organization and is independent of this crate-naming
convention. The two do not overlap:

- Crate naming reflects **where the crate lives in the workspace tree**.
- SEA layout reflects **how the crate is organized internally**.

A crate named `swe_edge_gateway` still has `src/api/`, `src/core/`,
`src/saf/` inside it.

## Rationale

- **Zero ambiguity.** Given a crate name, the path is recoverable by
  machine and by humans. Given a path, the name is forced.
- **No "magic" naming drift.** New members can't accidentally pick short,
  cute names that break the map.
- **Depth is a design signal.** Growing past 5 parts triggers a
  conversation, not a workaround.
