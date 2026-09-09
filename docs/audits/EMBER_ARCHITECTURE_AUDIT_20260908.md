# Ember Architecture Audit — 2026-09-08

Baseline audited: GitHub `main` commit `5d092565324d6726619a001331c6d5d86d3c50ea`.

## Executive result

The Ember repository is a valid GREEN seed, not yet a feature-complete editor/engine.

The ownership cutover is sound:

- Ember owns the active Rust workspace.
- Cortex implementation code is outside the active product.
- Havenwild/Open2D implementation is quarantined under `reference/donor`.
- The project-local Python Project Control Center is the authoritative root operations layer.
- GitHub CI and the local FULL quality gate are both GREEN.

The highest-value move is **not** to add broad features yet. The next phase should establish the missing architectural spines that every major editor/runtime feature will depend on.

## KEEP

1. `ember_core`
   - stable IDs
   - safe project-relative paths
   - diagnostics

2. `ember_project`
   - project identity
   - content roots
   - build-profile concept
   - enabled-package concept

3. `ember_documents`
   - central document envelope/registry concept

4. `ember_commands`
   - undo/redo command foundation

5. `ember_scene` / `ember_level`
   - Ember-owned editable scene/level authority
   - semantic cells and tile/entity concepts
   - command-driven edits

6. `ember_pixel`
   - independent pixel/animation document authority
   - layers/cels/frames/palettes
   - command-based edits

7. `ember_ldtk`
   - interchange-only LDtk parser/converter
   - Ember remains authoring authority

8. `ember_runtime`
   - authoring-to-runtime compile boundary
   - behavior execution proof

9. `ember_runtime_bridge`
   - temporary file-backed session proof
   - useful as a fallback transport/evidence format

10. `ember_cortex_adapter`
    - process/protocol boundary
    - no Cortex crate dependency

11. `reference/donor/havenwild_native_editor`
    - preserve as migration source
    - do not compile as active Ember authority

12. Project Control Center
    - local FULL GREEN gate
    - Git/GitHub authority
    - root-drop transactional patches
    - rollback/recovery
    - artifacts/debug bundles

## HARDEN

### Project/document persistence

Current persistence is too direct and weakly versioned.

Required:

- atomic save (`temp -> fsync/best effort -> replace`)
- backup/recovery policy
- explicit document schema version per concrete document type
- migration registry
- stable cross-document references
- dependency closure validation
- dirty/revision conflict semantics
- crash-recovery journal for editor documents

### Validation

Domain crates may own local validation rules, but only one project-level validation authority should aggregate them.

Required validation domains:

- project manifest
- document schema/version
- package resolution
- asset/provenance/dependency graph
- scene/level references
- behavior graphs
- runtime compilation
- build profiles
- editor workspace descriptors
- Cortex/PCC capability descriptors
- donor-boundary rules

### Capability truthfulness

Descriptors currently mix implemented and planned capabilities.

Every capability must have one state:

- `declared`
- `candidate`
- `certified`
- `unavailable`

Only `certified` capabilities may be advertised to Cortex or automation as executable.

### GitHub/reproducibility

Required:

- normalize repository description away from legacy ArbiterAI metadata
- decide public-vs-private policy for a workspace marked `Proprietary`
- branch/ruleset protection for `main` once workflow stabilizes
- toolchain policy (`rust-toolchain.toml` or explicit supported Rust/MSRV)
- line-ending policy (`.gitattributes`)
- fresh-clone certification in CI

## FILL GAP — P0

### 1. Native editor host

`apps/ember_editor` is currently a shell-model CLI.

Promote the proven Havenwild native editor incrementally rather than replacing it with a new framework.

Initial active path:

`ember_editor -> native host/backend -> FoundryShell/application services`

The native UI/render backend is a view/input adapter. It must not own documents, selection authority, undo, project state, or runtime state.

### 2. Application command/service spine

`ShellCommand` is currently descriptive.

Add:

- command registry
- command dispatcher
- enable/disable predicates
- shortcuts
- mutation transaction boundaries
- editor service locator/registry
- structured command results/events

UI, Cortex, menus, shortcuts, and automation must all invoke the same command authority.

### 3. Editor/runtime session + PIE

Replace the temporary file-only bridge with a typed local session protocol.

Must support:

- launch
- handshake
- ready
- load scene
- start/stop/pause/step
- hot reload
- snapshot
- selection/debug focus
- capture request/result
- structured logs/events
- crash/disconnect/reconnect
- graceful shutdown

File-backed JSON remains a fallback/evidence transport, not the live authority.

### 4. Background job system

Every heavy operation must go through one non-blocking job spine.

Jobs require:

- stable job ID
- type/owner
- queued/running/succeeded/failed/cancelled
- progress
- human status
- structured events
- cancellation
- log/artifact links
- parent/child jobs
- resource hints
- persisted recent history

### 5. Package/module system

`enabled_packages` exists but no resolver/contract exists.

Add package manifests, dependency/version constraints, capability contributions, content mounts, conflict diagnostics, migrations and enable/disable semantics.

Major engine/editor systems should be replaceable through these contracts instead of hard-coded feature branches.

### 6. Asset/Vault layer

Current asset registry is in-memory only.

Add:

- persisted asset catalog
- provider registry
- source-vs-derived authority
- provenance
- license/attribution
- dependency graph
- import transaction
- validation
- project-local content mounts
- shared Ember Vault mounts
- deterministic promotion from external/generated artifacts

### 7. Spatial model

Current scene transforms mix 3D position with 2D scale and scalar rotation.

Lock explicit modes:

- 2D
- 2.5D
- 3D

Use typed spatial transforms rather than one ambiguous transform for every document.

### 8. Behavior/node contract

Current node execution is a useful proof but not the final visual scripting architecture.

Add:

- typed ports
- value types
- execution/data edges
- event nodes
- functions/subgraphs
- deterministic scheduling
- execution budget
- diagnostics
- serialization/migrations
- gameplay/editor/runtime capability bindings
- animation events
- tool/action hooks
- test harness

### 9. Runtime systems spine

Do not build a giant engine yet.

First define replaceable runtime services:

- clock/fixed timestep
- input/action map
- scene/world lifecycle
- entity/component registry
- behavior execution
- resource resolver
- render adapter
- audio adapter
- physics/collision adapter
- save adapter
- networking adapter
- diagnostics

### 10. Integration-test project

Add a tiny original Ember certification project that exercises:

- project open/save
- one level
- tiles
- semantic cell
- entity
- behavior graph
- pixel asset
- LDtk import sample
- editor open
- PIE
- runtime tick
- snapshot
- validation
- packaging later

Do not use Havenwild as the only proof project.

## DEFER UNTIL P0 SPINES EXIST

- large terrain/world-generation feature expansion
- full GUI Studio
- full audio authoring
- broad IDE features
- marketplace/content-browser expansion
- production export matrix
- advanced 3D renderer work
- deep Cortex automation
- large plugin ecosystem
- large game templates

These should consume stable project/document/job/package/session contracts rather than define them accidentally.

## Naming cleanup

The active repository is Ember. Remaining `Open2D`, `O2D-*`, and `Foundry` names must be classified:

- persisted/versioned compatibility name: keep temporarily with migration
- donor/reference name: keep only under `reference/donor`
- active product label: rename to Ember
- internal concept deliberately retained (for example “Foundry” if chosen as a user-facing workspace): document explicitly

Do not perform a blind global rename.

## Architecture rule

A feature is not complete when a UI button exists.

It is complete when:

1. ownership is explicit,
2. the data contract is versioned,
3. the mutation path is transactional,
4. validation exists,
5. the runtime/editor boundary is defined,
6. the capability is certified,
7. tests prove the path,
8. diagnostics/artifacts are emitted,
9. the feature works without Cortex,
10. Cortex can use the same public operation later without private coupling.
