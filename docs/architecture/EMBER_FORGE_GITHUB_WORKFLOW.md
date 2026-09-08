# Ember Project Control Center + GitHub workflow

## Authority split

- GitHub: remote source history, PRs, issues, CI, releases.
- local Git: working tree and local commits.
- Ember Project Control Center (PCC): project-local operations, quality gates, transactions, patches, evidence, recovery, and GREEN-only Git workflow policy.
- Cortex: optional conversational/agent layer that can invoke the PCC CLI and Ember adapter.
- Universal/shared PCC tooling: optional higher-level orchestration; Ember does not require it to operate.

## Root contract

```text
PROJECT_CONTROL_CENTER.cmd              # authoritative root entry point
FORGE.cmd                               # compatibility alias -> PCC
project-control-center.profile.json     # declarative project profile
forge.project.json                      # compatibility/project capability contract
tools/control_center/ember_pcc.py       # self-contained Python implementation
```

The embedded PCC uses only the Python standard library plus project-native Git/Rust tools. It works from a fresh repository clone without Cortex or an external Forge runtime.

## Primary workflow

```text
1. FULL QUALITY GATE
2. COMMIT + PUSH FULL GREEN
```

The GREEN marker binds to the exact source fingerprint that passed format/check/test/Clippy and explicit editor/runtime builds. Commit/push refuses to proceed if source has changed since that gate.

## GitHub branch model

- `main`: certified integration authority.
- `migration/*`: large ownership/architecture cutovers.
- `feature/*`: normal feature work.
- `fix/*`: repairs.
- `archive/*`: preserved historical tips such as the pre-Ember repository state.

Avoid permanent `develop` unless the workflow proves it is needed.

## Root-drop updates

Ember patch ZIPs dropped at repository root are discovered at PCC startup only when they contain an Ember patch manifest. Paths and SHA-256 values are validated, touched files are backed up, changes apply transactionally, post-apply hashes are verified, failed transactions roll back, and consumed packages are archived under `artifacts/updates/`.

## GitHub CI

The included GitHub Actions workflow runs the portable Rust baseline. Local PCC FULL gates are intentionally stricter because they also build the editor/runtime and capture local evidence/debug artifacts.
