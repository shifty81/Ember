# Level Editor / Ember Boundary

Open2D Foundry owns editable authoring documents. Ember owns executable runtime state.

```text
Level Workspace
  -> LevelDocument
  -> validate
  -> compile_level
  -> RuntimeWorld
  -> Play / exported game
```

Authoring-only concerns such as display colors, editor definitions, layer organization, source provenance, selection, and undo history must not be required by exported runtime content.

LDtk import/export maps at the authoring boundary, never inside Ember.
