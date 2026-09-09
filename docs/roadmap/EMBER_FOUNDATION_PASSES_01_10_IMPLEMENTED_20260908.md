# Ember Foundation Passes 01–10 — Implementation Tranche

Baseline parent: `5e04e8f81dd92e5d887d763a35b8a87078e74114`.

This tranche implements the first compile-bounded form of all ten foundation passes before the next build.

## FND-01 Architecture truth normalization

- capability catalog with declared/candidate/certified/unavailable states
- Cortex descriptor no longer advertises unimplemented executable tools
- PCC is project-operations authority
- active O2D diagnostics normalized to EMBER namespace
- `EmberShell` becomes canonical shell name with compatibility alias
- architecture validator added

## FND-02 Document/storage foundation

- typed envelope encode/decode helpers
- migration registry
- atomic document writes
- atomic project manifest writes
- revisions advance during migrations
- dependency enumeration contract

## FND-03 Package/module foundation

- `ember_packages`
- package manifests
- dependency resolution and cycle detection
- content mount/provider contribution contracts

## FND-04 Background jobs

- `ember_jobs`
- threaded non-blocking jobs
- progress/status/cancellation/artifacts/diagnostics
- active-job queries and bounded wait for tests

## FND-05 Native editor promotion shell

- `ember_editor` is now a Macroquad native window host
- donor-proven host pattern retained: native window + panic log + frame loop
- renderer/input remain adapters over `EmberShell`
- workspaces/tabs/inspector/status/canvas shell are visible

## FND-06 Command/service/selection authority

- application command registry
- selection service
- shell dispatch path
- atomic save routing
- command transactions and rollback

## FND-07 Editor/runtime session and PIE foundation

- `ember_session`
- typed versioned JSONL protocol
- runtime process client
- runtime host stdio session mode
- load/play/pause/resume/step/stop/snapshot/shutdown commands
- unsupported hot reload/capture fail explicitly rather than pretending support

## FND-08 Asset/Vault foundation

- persistent asset catalog
- source revision/hash/provenance fields
- project/package/Vault/generated mount kinds
- dependency validation
- atomic catalog save

## FND-09 Authoring vertical slice

- original certification project includes pixel source, tileset record, level tile, semantic collision cell, entity and behavior graph
- integration test compiles level to runtime and ticks it

## FND-10 Hardened certification gate

- architecture validator wired into PCC FULL gate and GitHub CI
- certification integration test wired into both gates
- stable Rust toolchain policy recorded
- line-ending policy recorded

## Expected first build behavior

The first Cargo command after applying this tranche will update `Cargo.lock` because Macroquad and the new active workspace crates are now dependencies. That lockfile change is expected and must be included in the resulting GREEN commit.

Candidate capabilities remain candidate until this tranche passes the local FULL gate and GitHub CI. No capability should be promoted merely because code exists.
