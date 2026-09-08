# O2D-R005 — Unified Foundry Shell and Pixel Timeline

## Scope

This pass establishes an Open2D-owned, renderer-independent application shell. Havenwild remains a reference-only project and no Havenwild content or ownership assumptions enter the shell.

## Implemented

- `open2d_app_shell` crate.
- Project manifest loading and recent-project tracking.
- Open document tabs with active, dirty, pinned and close-state handling.
- Generic menus and shell commands.
- Persistent editor layout model.
- Status and diagnostic surfaces.
- Standard workspace registry for Project, World, Scene, Pixel & Animation, Nodes, GUI, Audio and Data.
- Automatic compatible-workspace selection when a document activates.
- Unified Pixel + Animation timeline snapshot.
- Undoable multi-pixel brush strokes.
- Undoable frame creation and frame-duration editing.
- Undoable palette color insertion.
- Updated `open2d_foundry` host using the shared shell.

## Boundary

The shell is the authority for project/document/workspace routing. The future native window and rendering backend will render this state through an adapter and must not own project state.

## Remaining for the native shell

- Native window lifecycle.
- GPU renderer backend.
- Docking and panel rendering.
- File dialogs.
- Canvas rendering and clipping.
- Visual timeline widgets.
- LDtk import dialog and result review.
- Project-browser UI.
- Layout persistence to disk.

## Acceptance commands

```bat
Open2DTools.cmd all
Open2DTools.cmd run
Open2DTools.cmd run path\to\project.open2dproject
```
