# Ember Foundation Passes 01–10

These passes precede broad feature expansion.

## EMB-FND-01 — Architecture truth normalization

- classify every project/Cortex capability as declared/candidate/certified/unavailable
- stop advertising unimplemented operations
- normalize PCC vs legacy Forge authority wording
- classify remaining Open2D/O2D/Foundry names
- add architecture validator

Exit: descriptors match executable reality.

## EMB-FND-02 — Document/storage foundation

- typed document codec registry
- schema versions
- migrations
- atomic writes
- revisions/fingerprints
- reference enumeration
- recovery journal contract

Exit: editor data can be safely persisted and migrated.

## EMB-FND-03 — Package/module foundation

- package manifest
- resolver
- content mounts
- capability/provider contributions
- dependency/conflict diagnostics

Exit: major systems have a replaceable composition mechanism.

## EMB-FND-04 — Background job spine

- `ember_jobs`
- process hosting
- progress/cancel/log/artifact events
- test harness

Exit: heavy editor operations have one non-blocking authority.

## EMB-FND-05 — Native editor promotion shell

- promote proven native Havenwild host behavior
- connect window/input/render to `FoundryShell`
- no gameplay/Havenwild ownership
- central layout, menus, tabs, status, panels

Exit: `ember_editor` opens a real native Ember window backed by Ember state.

## EMB-FND-06 — Command/service/selection authority

- application command registry + dispatcher
- shortcuts
- selection service
- document service
- workspace activation
- undo/redo transactions
- menu enablement

Exit: UI mutations flow through one testable path.

## EMB-FND-07 — Editor/runtime session + PIE

- `ember_session`
- runtime launch/handshake
- play/pause/step/stop
- snapshots/events
- reload
- capture
- crash/reconnect handling

Exit: F6/Play runs the same runtime host through a real local session.

## EMB-FND-08 — Asset/Vault foundation

- persisted asset registry
- providers
- import transaction
- source/derived authority
- provenance/licenses
- dependency validation
- Vault mount contract

Exit: project assets are discoverable, portable and traceable.

## EMB-FND-09 — Authoring workspace vertical slice

Promote one coherent path:

Pixel/Tileset -> Level -> PIE

- pixel document visible/editable
- publish tileset
- shared asset registry
- level paint
- semantic cell
- entity + simple graph
- save
- PIE

Exit: a tiny game scene can be created from inside Ember without hand-editing JSON.

## EMB-FND-10 — Certification project + hardened gate

- add original tiny certification project
- run editor/runtime integration tests
- validate capability descriptors
- validate donor isolation
- validate schemas
- fresh-clone/bootstrap CI lane
- branch/ruleset plan

Exit: GREEN means architecture and vertical workflow, not only crate compilation.

## After FND-10

Then expand:

1. Terrain Composer / semantic terrain
2. mature Pixel + Animation workflows
3. LDtk-style Level workspace parity
4. node system expansion
5. GUI/Data/Audio
6. 2.5D/3D spatial/runtime layers
7. IDE/code workspace
8. Cortex live integration
9. packaging/templates
10. marketplace/Vault ecosystem
