# Ember ownership and repository cutover

## Final repository model

```text
Ember/
├─ PROJECT_CONTROL_CENTER.cmd        # authoritative project-local root utility
├─ FORGE.cmd                         # compatibility alias -> PCC
├─ project-control-center.profile.json
├─ forge.project.json                # project capability/automation contract
├─ tools/control_center/ember_pcc.py
├─ Cargo.toml
├─ Cargo.lock
├─ apps/
│  ├─ open2d_foundry/                # TRANSITIONAL name; becomes Ember-owned Foundry surface
│  ├─ open2d_ember/                  # TRANSITIONAL name; Ember runtime/player
│  └─ haven_editor_native/           # migration/reference lane until generalized
├─ crates/
│  ├─ ember_runtime/
│  ├─ ember_cortex_adapter/          # Ember-owned, no Cortex crate dependency
│  └─ open2d_*/                      # transitional reusable crates, normalized incrementally
├─ integrations/
│  └─ cortex/
│     ├─ adapter.json
│     └─ schemas/
├─ tools/
│  └─ adapters/ember/                # Ember-specific Forge commands only
├─ docs/
├─ assets/
├─ content/
└─ .github/workflows/
```

## Must leave this repository

The following are Cortex product/source authorities and belong only in the standalone Cortex repository:

- `apps/cortex*`
- `apps/open2d_cortex*`
- `crates/cortex_*`
- `crates/open2d_cortex_*`
- `config/cortex/**`
- historical Cortex checkpoint/validator scripts whose only purpose is Cortex certification
- Cortex model-host/provider/service/desktop/CLI implementation
- Cortex-specific pass-marker gates and R051/Hxx product policy

Cortex history can be retained under migration archives outside the active workspace when useful, but must not participate in Ember builds or product identity.

## What remains

Ember keeps only project-side integration:

- `crates/ember_cortex_adapter`
- `integrations/cortex/adapter.json`
- editor IPC endpoints Cortex can call through the adapter
- Forge commands the adapter exposes to Cortex
- integration tests that prove Ember can work both with and without Cortex installed

## Dependency rule

Forbidden:

```text
ember_* -> cortex_*
open2d_* -> cortex_*
Ember editor -> Cortex Rust crate
Ember runtime -> Cortex service internals
```

Allowed:

```text
Ember -> stable adapter schema
Ember adapter -> Ember PCC CLI/API contract
Ember adapter -> Ember editor IPC
Cortex -> discovers/launches Ember adapter
Cortex -> Ember PCC machine/CLI surface
```

The adapter boundary must be transport/version negotiated and out-of-process capable.

## Naming migration

Do NOT combine detachment with a workspace-wide rename. First get a GREEN Ember-owned repository with Cortex gone. Then normalize names in bounded groups:

1. product/UI strings and root tooling: Open2D Foundry -> Ember/Ember Foundry as selected;
2. application package names;
3. reusable `open2d_*` crates where Ember is now authoritative;
4. reference-only Havenwild names;
5. persisted format namespaces only with explicit migrations.

This avoids turning a repository ownership change into a high-risk serialization/API rewrite.

## Required cutover gate

A cutover is GREEN only when:

- no active Cargo workspace member contains `cortex` except `ember_cortex_adapter`;
- no non-adapter Ember crate depends on a Cortex crate;
- Cortex configuration/product directories are absent from the active product tree;
- Ember can build/test with Cortex not installed;
- adapter schema validation passes;
- the embedded Ember PCC can discover/validate the repository and list/execute Ember capabilities;
- Git remote is `shifty81/Ember`;
- GitHub CI passes;
- no secrets/runtime `.cortex`, `.forge`, logs, targets, or generated debug bundles are tracked.
