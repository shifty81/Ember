# Ember

Ember is the standalone Rust game-authoring suite and runtime derived from the proven generalized Open2D Foundry/editor work.

## Repository ownership

This repository is Ember-owned. Cortex is a separate project and is integrated only through the Ember-owned adapter in `crates/ember_cortex_adapter` and `integrations/cortex/`.

Ember carries its own project-local Python Project Control Center. `PROJECT_CONTROL_CENTER.cmd` is the authoritative root operations entry point; `FORGE.cmd` remains only as a backward-compatible alias. Cortex and any universal/shared PCC installation are optional integrations, not runtime requirements.

## Active workspace

- `apps/ember_editor` — editor shell/product surface.
- `apps/ember_runtime_host` — Ember runtime host.
- `crates/ember_*` — generalized authoring/runtime crates promoted from the Open2D donor implementation.
- `crates/ember_cortex_adapter` — project-owned Cortex integration contract with no Cortex crate dependencies.

## Donor reference

`reference/donor/` is intentionally outside the Cargo workspace. It preserves high-value source/contracts needed during the next editor normalization passes, especially the mature Havenwild native editor and LDtk/editor authoring contracts. It is reference material, not an ownership dependency.

## First gate

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Remote authority: `https://github.com/shifty81/Ember`.


## Project Control Center

Run `PROJECT_CONTROL_CENTER.cmd` from repository root. The opening menu keeps the two primary operations first: **FULL QUALITY GATE** and **COMMIT + PUSH FULL GREEN**. The embedded PCC also owns Git/GitHub repair, build/run, dependency checks, root-drop transactional patch intake, recovery, logs, debug bundles, and source snapshots under `artifacts/`.


## Foundation tranche

The post-cutover FND-01 through FND-10 architecture tranche is tracked in `docs/roadmap/EMBER_FOUNDATION_PASSES_01_10_IMPLEMENTED_20260908.md`. Candidate capabilities are promoted to certified only after local FULL GREEN and GitHub CI evidence.
