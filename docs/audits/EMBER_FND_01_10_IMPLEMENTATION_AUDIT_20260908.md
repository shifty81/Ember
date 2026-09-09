# Ember FND-01–10 Implementation Audit — 2026-09-08

Parent baseline: `5e04e8f81dd92e5d887d763a35b8a87078e74114`.

This audit distinguishes **implemented foundation** from **certified behavior**. New runtime/editor capabilities remain `candidate` until the first local FULL gate and subsequent GitHub CI pass.

| Pass | Implementation | Initial evidence before build | Certification target |
|---|---|---|---|
| FND-01 | Capability catalog, PCC authority normalization, active-name validator, Cortex truth projection | JSON/TOML/Python static parse | architecture validator + Clippy + CI |
| FND-02 | Typed envelope helpers, migration registry, atomic document/project/level writes | source review | document/project tests + workspace tests |
| FND-03 | `ember_packages`, dependency resolver, mounts/providers | unit tests authored | workspace tests |
| FND-04 | `ember_jobs`, progress/cancel/log/artifact state, process jobs | unit tests authored | workspace tests |
| FND-05 | Macroquad native Ember editor host using `EmberShell` | donor API cross-check | Windows editor build + launch test |
| FND-06 | command registry, selection service, shell dispatch, transactional command batch | unit tests authored | workspace tests + editor interaction |
| FND-07 | `ember_session`, JSONL protocol, runtime process client, runtime stdio host, editor F6 connection | protocol tests authored | runtime/editor builds + PIE smoke |
| FND-08 | persistent asset catalog, mount kinds including Vault, dependency validation | unit tests authored | workspace + certification test |
| FND-09 | original certification project: pixel, tile, semantic cell, entity, behavior graph, runtime compile/tick | fixture static validation | certification integration test |
| FND-10 | architecture + certification steps wired into local FULL gate and GitHub CI | PCC Python syntax/status | FULL gate + GitHub CI |

## Known candidate boundaries after this tranche

These are intentionally **not** represented as certified yet:

- native editor usability beyond the shell/viewport foundation;
- named-pipe transport;
- live document/asset hot reload;
- rendered runtime capture;
- package UI/management;
- Vault UI and remote/shared storage workflows;
- full Pixel/Tileset/Level authoring UI vertical slice;
- Terrain Composer;
- GUI/Audio/IDE production workspaces;
- Cortex executable transport/plugin host.

The implementation is designed so those features extend the new contracts rather than establish competing authorities.

## First build expectations

1. `cargo fmt --all` runs first and may normalize the new Rust files.
2. Cargo resolves the new Macroquad/native-editor dependencies and updates `Cargo.lock`.
3. Cargo check/test/Clippy then certify the source.
4. Architecture validation verifies boundary truth.
5. Certification test verifies the original Ember smoke project.
6. Editor and runtime host are linked as real executables.
7. If any step fails, PCC emits a debug bundle and the source remains uncommitted.
