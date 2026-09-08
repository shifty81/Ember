# Ember Native Editor Promotion Contract

## Locked identity

Open2D Foundry is the overall development ecosystem.

- **Cortex** — AI/project orchestration, chat, memory, project operations, intake.
- **Ember** — native engine/editor experience and infinite authoring canvas.
- **ember_runtime** — runtime library used by Ember and shipped games.
- **Ember Player / Runtime Host** — internal runtime-launch executable; this is the future
  user-facing clarification for the current `open2d_ember` runtime CLI.

## Existing migration source

The Open2D source tree already contains the Havenwild native editor reference at:

`apps/haven_editor_native`

and Havenwild reference/support crates. This source is a migration seed, not final Open2D authority.

The promoted active application will be:

`apps/ember_editor_native`

The Havenwild reference app remains untouched until Ember reaches parity.

## Promotion rule

Do not copy Havenwild gameplay rules into Ember.

Promote editor behavior and authoring infrastructure:
- infinite/native canvas behavior;
- camera/zoom/pan;
- selection;
- workspace chrome;
- docking/panel concepts;
- scene outliner;
- scene tool rail;
- asset palette/browser;
- object inspector;
- world/scene authoring interaction;
- Pixel Studio shell integration points;
- Animation Studio shell integration points;
- GUI Studio shell integration points;
- undo/redo command flow;
- editor logging/crash recovery;
- runtime/dev preview bridge patterns.

Do not promote as engine authority:
- Havenwild progression;
- farm/tavern rules;
- Havenwild maps;
- quests/NPC/crops/recipes;
- Havenwild-only content IDs;
- Havenwild-specific worldgen profiles;
- direct `HAVENWILD_*` environment variables;
- Havenwild-specific client executable assumptions;
- Havenwild asset pack policy where it is game-specific.

## File promotion lanes

### Lane A — preserve almost directly
- `canvas_camera.rs`
- `canvas_controller.rs`
- `canvas_view.rs`
- `selection_controller.rs`
- generic render helpers
- generic editor text/theme primitives after token normalization

### Lane B — extract/generalize
- `workspace_shell.rs`
- `workspace_chrome.rs`
- `editor_menu.rs`
- `scene_outliner.rs`
- `scene_toolrail.rs`
- `object_inspector.rs`
- asset library/palette/intake panels
- sprite/pixel canvas authority
- GUI Studio shell

### Lane C — adapt to Open2D documents/runtime
- scene authoring
- world surface editing
- development session/client bridge
- production tools
- building/world previews
- structural authoring helpers

### Lane D — reference only / do not promote wholesale
- Havenwild-specific island/world rules
- Havenwild development-world schemas
- Havenwild content paths
- Havenwild-specific client commands

## Infinite canvas contract

Ember's center surface is an infinite workspace that can host typed authoring documents/items.

Initial item types:
- Level/Scene
- Sprite
- Tileset
- Animation
- GUI
- Node Graph
- Image/Concept
- Document/Reference
- Build/Runtime Preview
- Cortex Chat/Task reference

Items have:
- stable ID;
- type;
- transform;
- z-order;
- source asset/provenance;
- selection state;
- tool binding;
- project ownership;
- saved workspace placement.

The canvas is not merely a whiteboard. Typed items open their native tool/workspace when activated.

## Cortex -> Ember generated asset flow

When Cortex image generation is available:

```text
Prompt / Cortex task
 -> generated image artifact
 -> provenance + project tag
 -> preview in Cortex
 -> Send to Ember Canvas
 -> Ember creates Image/Concept canvas item
 -> user may promote it into:
      Sprite
      Tileset
      Animation source
      GUI asset
      Level reference
      Pixel Studio document
```

No generated image becomes a production asset silently.

## Tool bridge

Cortex and Ember communicate through stable commands/events rather than private UI coupling.

Examples:
- ember.open_project
- ember.open_canvas
- ember.open_asset
- ember.focus_item
- ember.import_artifact
- ember.create_canvas_item
- ember.publish_asset
- ember.validate_asset
- ember.play_scene
- ember.capture_view
- ember.get_selection

This allows Cortex to generate or retrieve content and place it directly into the correct tool/canvas.

## Migration sequence

1. Create `apps/ember_editor_native` from the existing Havenwild native-editor seed.
2. Rebrand title/log/schema namespaces only.
3. Keep the current canvas behavior intact.
4. Introduce Open2D project/document adapters.
5. Replace Havenwild environment/client assumptions.
6. Promote generic shell/panel/input code.
7. Migrate scene/world document authority to Open2D crates.
8. Add Cortex command bridge.
9. Add typed infinite-canvas item registry.
10. Retire `apps/haven_editor_native` reference only after parity and acceptance.
