# Ember Foundation Closure V1

Baseline: `674abe357a88250876f4216b35d59295416a87ef`

This roadmap closes the shared engine/editor spines before broad workspace UI work.

## Current pass

### EMBER-CLOSE-01A — Authority & truth

- explicitly enumerate every active Ember workspace crate;
- pin the known-GREEN Rust 1.95.0 toolchain locally and in CI;
- keep donor/reference code outside active workspace authority;
- correct `terrain.composer` ownership to `ember_terrain`;
- stop advertising `run.game` before a visual runtime exists;
- truthfully label the current runtime executable as a headless runtime service;
- remove root `tools.zip`;
- ignore root operational patch/archive residue;
- strengthen architecture validation around these rules.

This pass does **not** add new editor features or third-party runtime/editor dependencies.

## Next

- CLOSE-01B dependency/license policy + runtime/shell smoke certification
- CLOSE-02 production editor substrate spike (winit/wgpu/egui/egui_dock)
- CLOSE-03 type/property registry
- CLOSE-04 document session/history/recovery
- CLOSE-05 resource/import/cache/dependency pipeline
- CLOSE-06 prefab/instance overrides
- CLOSE-07 jobs V2
- CLOSE-08 session V2
- CLOSE-09 runtime kernel
- CLOSE-10 renderer
- CLOSE-11 physics/audio providers
- CLOSE-12 deep certification lanes
- CLOSE-13 gameplay API/package semantics
- CLOSE-14 project/filesystem service
- CLOSE-15 Ember Code Lite + Forge bridge

Only then promote the first complete Level -> Inspector -> Save -> PIE -> Render workflow.
