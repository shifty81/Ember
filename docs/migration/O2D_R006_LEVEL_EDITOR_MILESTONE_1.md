# O2D-R006 — Level Editor Milestone 1

## Purpose
Establish the Open2D-owned authoring/runtime spine needed to rebuild LDtk-class level editing inside Foundry without making LDtk the runtime or internal data authority.

## Added
- `open2d_level` native level-authoring crate.
- Open2D Level document kind and Level workspace routing.
- Tile layer and SemanticGrid layer construction helpers.
- Undoable add-layer, tile-paint, semantic-paint, and entity-placement commands.
- Semantic definitions that bind authoring values to collision/navigation/tags/behaviors.
- JSON save/reload for native `.open2dlevel`-style documents.
- Ember level compiler that strips editor-rich data to runtime entities and semantic cells.
- Runtime behavior resolution through an explicit behavior library.
- `Open2DTools.cmd level-smoke` vertical-slice verification command.

## Architectural boundary
LDtk remains an MIT-licensed reference and compatibility adapter. The Level Editor edits `open2d_level::LevelDocument` directly. Ember consumes compiled runtime state and never depends on LDtk.

## Acceptance target established
The vertical slice now has code paths for:
1. Create a native level document.
2. Add tile and semantic layers.
3. Paint authored data through command history.
4. Undo and redo edits.
5. Save to JSON and reload it.
6. Compile the reloaded level into Ember runtime state.

## Environment validation
This execution environment does not provide Rust/Cargo, so full compile/Clippy/test certification remains a Windows-machine gate. Cargo manifests and JSON inputs were structurally parsed here.
