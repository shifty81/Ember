# Donor KEEP / PORT / DROP Matrix

| Donor area | Ember disposition |
|---|---|
| generalized `open2d_core/project/assets/documents/scene/level/commands` | KEEP + renamed to `ember_*` |
| generalized editor/canvas/pixel/world/nodes/LDtk/tooling/runtime bridge | KEEP + renamed to `ember_*` |
| `ember_runtime` | KEEP as active authority |
| `apps/open2d_foundry` | PORT to `apps/ember_editor` |
| `apps/open2d_ember` | PORT to `apps/ember_runtime_host` |
| Cortex implementation apps/crates/config | DROP from Ember; standalone Cortex owns them |
| Open2D↔Cortex compatibility facade crates | DROP; replace with `ember_cortex_adapter` |
| Havenwild game/runtime/content | DROP from active Ember |
| Havenwild native editor implementation | QUARANTINED DONOR REFERENCE for deliberate porting only |
| LDtk integration/reference | KEEP/PORT |
| historical Hxx/R051 Cortex marker validators | DROP |
| Universal PCC/Forge implementation | EXTERNAL authority; keep only resolver + project contract |
| GitHub repository | `shifty81/Ember` authoritative remote |
