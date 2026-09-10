# Ember Code Lite / Forge IDE Boundary

**Ember owns game authoring. Forge owns full software development.**

Ember's current `ember_ide` crate is a lightweight source-model foundation and must not grow into a second full IDE.

Ember Code Lite will eventually own ordinary game-authoring edits: scripts, shaders, project data/config, line navigation, syntax highlighting, basic find/replace, undo/redo/save, diagnostics, game API completion, script attachment, and **Open in Forge**.

Forge owns LSP/DAP lifecycle, terminals, Git/diff/merge, repository-wide refactors/search, full debugger/profiler/test explorer, dependency/build-system editing, and Cortex software-development workflows.

Code Lite remains usable when Forge is unavailable. Forge-enhanced completion, diagnostics, navigation and code actions are capability-gated optional services.

A future bounded migration may rename `ember_ide` to `ember_source`; CLOSE-01A records this ownership decision but deliberately does not rename the crate.
