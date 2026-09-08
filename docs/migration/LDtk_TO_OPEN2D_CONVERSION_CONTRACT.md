# LDtk to Open2D Conversion Contract

LDtk is an import/interchange provider. Open2D owns the resulting documents.

```text
LDtk JSON
  -> open2d_ldtk parser
  -> import profile
  -> open2d_scene editable documents
  -> validation and user review
  -> compiled Ember runtime products
```

## Rules

1. Imported data never becomes runtime state directly.
2. Unknown source fields are preserved where practical.
3. IntGrid values are semantic inputs, not fixed tile IDs.
4. Entity fields map into generic components through explicit profiles.
5. External levels remain explicit dependencies and cannot silently disappear.
6. Reimport must eventually operate through a source record and reviewable diff.
7. Open2D identifiers and documents remain independent from LDtk application internals.
