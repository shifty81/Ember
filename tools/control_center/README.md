# Ember Project Control Center

`PROJECT_CONTROL_CENTER.cmd` is the authoritative project-local operations entry point for Ember.
It uses `tools/control_center/ember_pcc.py` and Python's standard library only. Cortex and any shared/universal control-center runtime are optional integrations; Ember does not require them to build, validate, patch, recover, or operate its repository.

## Primary commands

```powershell
PROJECT_CONTROL_CENTER.cmd --status
PROJECT_CONTROL_CENTER.cmd --full-gate
PROJECT_CONTROL_CENTER.cmd --commit-push-green
PROJECT_CONTROL_CENTER.cmd --git-repair
PROJECT_CONTROL_CENTER.cmd --patch-intake
PROJECT_CONTROL_CENTER.cmd --debug-bundle
PROJECT_CONTROL_CENTER.cmd --source-snapshot
```

Launching `PROJECT_CONTROL_CENTER.cmd` with no arguments opens the streamlined interactive menu.

## Full quality gate

The FULL gate performs:

1. Root/project manifest audit.
2. Git/Rust dependency preflight.
3. `cargo fmt --all -- --check`.
4. `cargo check --workspace --all-targets`.
5. `cargo test --workspace`.
6. strict workspace Clippy.
7. explicit editor build.
8. explicit runtime-host build.
9. source fingerprint capture and GREEN evidence under `artifacts/quality-gates/`.

`Commit + Push FULL GREEN` refuses to commit when the current source fingerprint differs from the last passing FULL gate.

## Root-drop update packages

The PCC scans repository-root ZIP files at startup. A ZIP is treated as an Ember update only when it contains one of these manifests at ZIP root:

- `EMBER_PATCH_MANIFEST.json`
- `ember-patch.json`
- `patch-manifest.json`

Manifest schema:

```json
{
  "schema_version": 1,
  "project_id": "ember",
  "patch_id": "EMBER-PATCH-001",
  "files": [
    {
      "path": "relative/project/file.rs",
      "sha256": "64-lowercase-hex-characters"
    }
  ],
  "remove": ["optional/obsolete/file.rs"]
}
```

Payload files are stored at `payload/<relative/project/path>` in the ZIP. The PCC validates safe paths and SHA-256 values before applying. Changed files are backed up under `artifacts/recovery/`, application is verified, consumed packages are archived under `artifacts/updates/consumed/`, and failed transactions roll back automatically.

## Artifacts

Generated operational state stays under ignored `artifacts/`:

- `artifacts/logs/sessions/`
- `artifacts/quality-gates/`
- `artifacts/debug-bundles/`
- `artifacts/source-snapshots/`
- `artifacts/updates/`
- `artifacts/recovery/`

A failed FULL gate or update automatically creates an upload-ready debug bundle. Interactive Windows runs also open the debug-bundle folder on failure.
