# Ember System Architecture V1

## Product identity

Ember is one integrated Rust game-authoring suite and runtime.

It supports 2D, 2.5D and 3D authoring through shared project, document, command, asset, package, validation and runtime contracts.

Cortex is external. The Project Control Center is project-local operations authority.

## Layering

```text
+------------------------------------------------------------------+
|                        apps/ember_editor                         |
| Native window/input/render host + workspace composition          |
+-----------------------------+------------------------------------+
                              |
                              v
+------------------------------------------------------------------+
| Editor application services                                      |
| shell | command dispatch | selection | documents | jobs | layout |
+------------------------------------------------------------------+
   |             |              |              |             |
   v             v              v              v             v
documents     assets         packages        jobs        validation
   |             |              |              |             |
   +-------------+--------------+--------------+-------------+
                              |
                              v
+------------------------------------------------------------------+
| Authoring domains                                                 |
| level/scene | pixel/animation | terrain/world | nodes | GUI/etc. |
+------------------------------------------------------------------+
                              |
                         validate/compile
                              |
                              v
+------------------------------------------------------------------+
| Runtime contract                                                  |
| scene lifecycle | systems | resources | behaviors | render/etc.  |
+------------------------------------------------------------------+
                              |
                              v
+------------------------------------------------------------------+
| apps/ember_runtime_host                                           |
+------------------------------------------------------------------+

External/process boundaries:

Cortex <-> ember_cortex_adapter <-> PCC machine API / editor IPC
Editor <-> Ember Session Protocol <-> Runtime Host
Vault  <-> Asset Provider/Mount contract <-> Project
```

## Ownership rules

### `ember_core`

Owns only primitive cross-cutting contracts:

- StableId
- ProjectPath
- Diagnostic
- protocol/schema identifiers
- small value types that truly span domains

It must not become a dumping ground.

### `ember_project`

Owns:

- project manifest
- project format version
- build profiles
- content roots
- enabled packages
- project settings references

It does not own actual assets/documents/runtime state.

### `ember_documents`

Owns:

- document identity
- document metadata
- document registry
- codecs/migrations registry
- revision/dirty/save semantics

Concrete typed payloads remain owned by domain crates.

### `ember_commands`

Owns mutation history and transaction semantics.

All authoring mutations that should support undo/redo flow through commands/transactions.

### `ember_jobs`

Planned foundation crate.

Owns background-work lifecycle and observable progress. It does not own domain-specific work.

### `ember_packages`

Planned foundation crate.

Owns package/module manifests, resolution, mounts and capability contributions.

### `ember_assets`

Owns asset catalog/provider/provenance/dependency/import contracts.

Actual pixel, scene, graph and other authoring models remain in their domain crates.

### `ember_editor_core`

Owns editor application services and workspace registration, but not native window/render APIs.

### Native editor backend

Owns:

- window
- input events
- rendering
- OS dialogs/clipboard
- platform integration

It adapts to editor services. It never becomes project/document authority.

### `ember_runtime`

Owns executable state and deterministic runtime scheduling.

It consumes compiled authoring data, not editor-only data structures.

### `ember_session`

Planned foundation crate.

Owns editor/runtime local protocol and session lifecycle.

### `ember_cortex_adapter`

Owns only the external protocol projection.

It does not reimplement project operations or editor internals.

## Replaceability rule

Replaceable systems use:

`contract -> registry -> selected provider -> capability certification`

Examples:

- renderer
- audio
- physics
- navigation
- terrain generator
- asset provider
- save backend
- networking
- image generator
- external editor/import provider

No major subsystem should require changing unrelated core crates merely to swap its implementation.

## Dependency direction

Preferred:

```text
core
 ^ 
project/documents/commands/jobs/packages
 ^ 
assets + authoring domains
 ^
editor_core / runtime
 ^
apps
```

External adapters depend inward on stable contracts.

Forbidden:

- core -> editor UI
- runtime -> editor UI
- authoring domain -> Cortex implementation
- package provider -> private PCC implementation
- native UI backend -> ownership of serialized project state

## Certification principle

Compile success is necessary but not sufficient.

Each externally advertised capability must be backed by:

- implementation
- validator
- tests
- structured result
- failure diagnostics
- versioned contract
- capability status = `certified`
