# Ember CLOSE-01B1 — Certification Evidence

Baseline: `375be44e3d80ecd836f9fb4dac5ed74df61577d1`

This subpass deliberately adds evidence **before** capability promotion.

It adds three independent executable checks to the existing workspace test gate:

- `apps/ember_runtime_host/tests/stdio_session_smoke.rs`
  - spawns the actual runtime-host binary;
  - waits for `Ready` with a bounded timeout;
  - creates and loads a behavior-free temporary Level;
  - Snapshot -> Play -> Step -> Stop -> Shutdown;
  - captures stderr;
  - kills the child on abnormal test exit.

- `crates/ember_app_shell/tests/shell_truth.rs`
  - proves path-backed document save;
  - proves successful save clears dirty state;
  - proves saved content reloads;
  - proves dirty close refuses unsafe data loss;
  - proves Save All does not falsely save pathless documents.

- `crates/ember_tools/tests/dependency_policy.rs`
  - reads Cargo metadata under `--locked`;
  - rejects third-party Git dependencies;
  - rejects external path dependencies outside workspace authority;
  - restricts registry sources to approved crates.io registry identities;
  - requires license metadata or license file for every third-party package.

No capability is promoted in this patch. If the FULL gate passes, the next
small CLOSE-01B2 patch may promote only the capabilities directly supported by
the passing evidence.

This pass does not add wgpu, egui, bevy, Rapier, Kira, Tree-sitter, or other
production editor/runtime providers.
