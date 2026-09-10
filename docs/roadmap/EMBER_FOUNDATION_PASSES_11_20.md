# Ember Foundation Passes 11–20

Baseline: GitHub `main` commit `12e52cb2a2ac9d487e231e290b0140c88cddaf24`.

These passes turn the FND-01–10 architectural spines into reusable authoring backends.
They intentionally avoid a broad native-editor UI rewrite in this tranche; the next
UI pass consumes these APIs after they are compile/test certified.

## EMB-FND-11 — Terrain Composer semantic backend

- `ember_terrain`
- land/structure semantic cells
- explicit surface roles and elevation
- cardinal autotile masks/rules
- separate water layer contract

Exit: terrain semantics and water authority are cleanly separated and autotile-ready.

## EMB-FND-12 — Animation publication backend

- `ember_animation`
- PixelDocument frame-tag -> AnimationClip publication
- playback modes
- animation events
- socket keyframes
- hitbox keyframes
- sprite-sheet layout validation

Exit: Pixel/Animation authoring can publish structured runtime/editor metadata.

## EMB-FND-13 — Native Level authoring commands

- `ember_level_tools`
- rectangle tile fill
- layer reorder
- entity property mutation
- undo-compatible command implementations

Exit: common level-editor mutations are reusable commands rather than UI-local logic.

## EMB-FND-14 — Typed Node/Logic graph

- `ember_graph`
- typed execution/data ports
- connection validation
- value-type compatibility
- ordered execution edges
- compilation into current BehaviorGraph runtime IR

Exit: visual scripting has a typed authoring contract above the simple executable graph.

## EMB-FND-15 — GUI/Data/Audio typed documents

- `ember_studios`
- GUI widget tree and layout validation
- typed data tables
- audio-event layers/parameters

Exit: the three studios have real serializable domain models ready for workspace UI.

## EMB-FND-16 — Explicit 2D / 2.5D / 3D spatial model

- `ember_spatial`
- dedicated transform types
- explicit spatial mode
- orthographic-depth projection profile

Exit: new authoring systems stop depending on one ambiguous hybrid transform.

## EMB-FND-17 — Integrated IDE/source model

- `ember_ide`
- source language classification
- project-relative source index
- deterministic source fingerprint
- UTF-8-safe non-overlapping text edits

Exit: code/source authoring has a project-aware backend independent of a particular text widget.

## EMB-FND-18 — Certified external bridge routing

- `ember_bridge`
- tool -> capability -> owner route
- certified-only execution rule
- Cortex adapter handshake projection

Exit: Cortex/automation can consume public Ember operations without private editor coupling.

## EMB-FND-19 — Cook / packaging plan

- `ember_packaging`
- deterministic cook entries
- duplicate-output validation
- release manifest generation
- existing BuildTarget integration

Exit: packaging has a versioned deterministic plan before archive/installer implementation.

## EMB-FND-20 — Library / Vault / template contracts

- `ember_library`
- versioned library entries
- project/Vault locators
- dependency validation
- explicit promotion plans
- project template file mapping

Exit: shared content and templates have one reusable contract instead of ad-hoc copying.

## Certification

`crates/ember_tools/tests/foundation_v2.rs` crosses all ten passes in one test so the
workspace gate proves the APIs compose, not merely compile in isolation.

## After FND-20

The next tranche should be native editor consumption:

1. Level workspace document open/create/save/reopen
2. interactive tile/semantic painting
3. Pixel + Animation workspace rendering/tools
4. Terrain Composer workspace
5. typed Node/Logic workspace
6. inspector/outliner/property editors
7. GUI/Data/Audio workspace shells
8. source/IDE workspace
9. live PIE reload and selection/debug bridge
10. package/Vault/cook UI and first standalone package certification
