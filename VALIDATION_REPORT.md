# Seed Validation Report

Generated 2026-09-08.

## Passed static checks

- Every active Cargo workspace member exists.
- Every active local path dependency resolves to an included crate.
- Active package names are Ember-owned (`ember_*`).
- No active application/crate depends on a Cortex implementation crate.
- The Cortex integration is isolated to `ember_cortex_adapter` and `integrations/cortex`.
- Havenwild native-editor material is quarantined under `reference/donor` and is not a Cargo workspace member.
- Ember includes a self-contained project-local Python PCC with root launcher, GREEN gate, Git/GitHub authority repair, transactional patch intake/recovery, diagnostics, and artifact handling.

## Runtime/build certification

The packaging environment did not have the Rust `cargo` executable installed, so compilation, tests, rustfmt and Clippy could not be executed here. The embedded PCC itself passed Python syntax/status smoke checks. Run the first FULL GREEN gate on the target Windows/Rust development machine:

```powershell
PROJECT_CONTROL_CENTER.cmd --full-gate
```

Any resulting failures should be treated as donor-port normalization work inside the new Ember repository, not repaired back in O2DF.
