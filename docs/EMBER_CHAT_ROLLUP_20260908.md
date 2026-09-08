# Ember Chat Rollup — 2026-09-08

## Final ownership decision

Ember is now the sole owner of this repository/product line. The prior O2DF/Open2D repository is donor material only and can be retired after this seed is safely stored and verified.

Cortex is a standalone local-first AI/development platform. No `cortex_*` implementation crate belongs in Ember. Ember owns only its adapter/plugin contract and integration descriptors.

Ember includes its own project-local Python Project Control Center so a fresh clone is operational by itself. Universal PCC/Cortex tooling may orchestrate Ember through formal interfaces, but neither is required for Ember root operations.

GitHub repository authority: `https://github.com/shifty81/Ember`.

## Ember product direction retained from project planning

- One integrated Rust 2D/3D game-authoring application rather than disconnected tools.
- Editor-first game creation: routine authoring should happen inside the editor and be directly testable through embedded play/runtime workflows.
- Major systems must be modular and replaceable through stable contracts, registries, packages, profiles, and adapters instead of hard-coded engine dependencies.
- The mature pixel workflow is retained: LibreSprite-style tools, layers/cels/frames, palettes, animation timeline/tags, sprite-sheet/atlas workflows, selections/transforms, brushes/fill, onion skinning, events, anchors/sockets, hitboxes/hurtboxes, metadata, validation/publish, source-vs-derived authority, provenance, and `.ase/.aseprite` compatibility targets.
- Levels remain the rightmost major authoring workspace; Terrain Composer sits between Pixel/Tilesets and Levels.
- LDtk-style level authoring is a native secondary workspace with shared tilesets, levels/worlds/layers/entities/IntGrid/auto-layer concepts.
- Semantic terrain brushes paint coherent land, water, cliffs/elevation, paths/roads and transitions while derived visual layers resolve automatically and non-destructively where possible.
- Terrain/world generation supports land-only semantic terrain with water as an independent render/simulation system, erosion/elevation/slope/drainage/watersheds/rivers/lakes/coastlines/sediment/biome masks.
- Heavy operations must be non-blocking background jobs with visible progress, logs, cancellation and artifacts.
- Generated images should become internal artifact cards/workflow objects rather than loose downloads.
- Shared reusable logic/assets belong in a versioned Vault/library model with typed ports/schemas, dependencies, provenance, tests and compatibility metadata.
- Source/code IDE editing is part of the authoring suite and shares the same project registry, command bus, validation, build console, Git/diff, AI and workspace model.

## Cortex integration boundary

Ember communicates with standalone Cortex using the Ember-owned adapter. The public boundary should be process/protocol based, not Rust trait-object ABI coupling. Preferred interoperability includes typed JSON/JSONL and MCP-style tool/resource contracts where applicable. Cortex may inspect projects, invoke Forge operations, edit source through governed transactions, run jobs, generate/ingest artifacts and interact with editor/runtime capabilities exposed by Ember.

## Project Control Center boundary

The embedded Ember root utility owns Ember build/test/validation orchestration, GREEN evidence, logs/debug bundles, root-drop patch intake, transactional apply/rollback, recovery snapshots, source snapshots, Git/GitHub operations, and project health. Higher-level universal tooling and Cortex integrate through the project contract/CLI rather than replacing this repository-local authority.

## Migration rule

Do not copy Cortex internals or Havenwild game/domain ownership into active Ember source. The quarantined `reference/donor` tree exists solely so proven editor implementations can be ported deliberately, normalized and then deleted when parity is achieved.
