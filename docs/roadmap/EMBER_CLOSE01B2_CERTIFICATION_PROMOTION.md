# Ember CLOSE-01B2 — Certification Promotion

Authority baseline: `375be44e3d80ecd836f9fb4dac5ed74df61577d1`

CLOSE-01B1 applied successfully. Its aggregate gate stopped at strict Clippy
because one test used `assert_eq!(bool, true)`, but the complete workspace test
stage passed, including:

- runtime-host stdio subprocess certification;
- editor shell/save truth certification;
- dependency-origin/license metadata policy.

This B2 patch:

1. fixes only that Clippy assertion;
2. promotes `runtime.session.stdio` from Candidate to Certified;
3. promotes `editor.save` from Candidate to Certified;
4. keeps `editor.native_host` and `editor.play_test` unpromoted;
5. adds a capability-truth test requiring every Certified capability to carry
   evidence and to depend only on other Certified capabilities;
6. records Dependency Policy, Runtime Service, and Editor Model as certified
   evidence lanes;
7. marks CLOSE-01B complete only under the rule that this patch is committed
   solely after its requested FULL QUALITY GATE passes.

No new GUI, renderer, ECS, physics, audio, scripting, or asset-pipeline provider
is introduced here.

After GREEN + commit, the next Ember pass is CLOSE-02: isolated production
editor-substrate certification, with the existing Macroquad editor retained
until parity is proven.
