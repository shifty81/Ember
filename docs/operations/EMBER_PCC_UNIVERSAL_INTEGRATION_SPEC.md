# Ember — Universal Project Control Center Requirements

Status: project-specific integration authority for the universal Project Control Center (PCC), Cortex automation, and Ember's project-local PCC.

This document separates **universal PCC behavior** from **Ember-specific behavior**. The universal PCC owns the common operation host, patch transport, transaction model, logs, handoffs, source-control integration, and project registry. Ember supplies a machine-readable `project.control.json` that declares the exact commands and gate stages the universal PCC must use instead of guessing from `Cargo.toml`.

## 1. Authority model

For Ember, source authority is:

`Git working tree -> current Git HEAD/remote -> project.control.json -> PROJECT_CONTROL_CENTER.cmd -> tools/control_center/pcc_bootstrap.py -> Ember project operations`

The committed Git repository is the baseline. A debug/handoff bundle is an overlay describing the local working tree and failed operation. Old source rollups are reference/recovery material only and must never silently become the development baseline.

Repository authority:

- Project ID: `ember`
- Project name: `Ember`
- Repository: `https://github.com/shifty81/Ember.git`
- Default branch: `main`
- Root launcher: `PROJECT_CONTROL_CENTER.cmd`
- Project-local PCC bootstrap: `tools/control_center/pcc_bootstrap.py`
- Legacy/project implementation behind the bootstrap: `tools/control_center/ember_pcc.py`
- Capability catalog: `config/ember/capabilities.json`
- Cortex boundary: `integrations/cortex/adapter.json`

## 2. Universal PCC project discovery

The universal PCC must prefer explicit project contracts in this order:

1. `project.control.json`
2. an explicitly registered adapter/provider
3. project-local PCC launcher/profile
4. generic language/build-system inference only as a final fallback

When `project.control.json` exists, the PCC must not replace `gate.full`, `patch.apply`, build, run, validation, or release commands with inferred Cargo commands.

The GUI may present universal buttons, but the button must dispatch the command ID declared by the project. The GUI must never contain a second hidden implementation of Ember's build/gate logic.

## 3. Mandatory pre-operation patch stage

Before any operation that can build, test, run, package, certify, commit, push, migrate, or otherwise depend on current source, the universal operation host must perform:

`discover patches -> validate target -> stage -> verify -> apply transactionally -> reload project/PCC provider if changed -> resume requested operation`

A project showing `Updates: N pending` must never begin `cargo`, `cmake`, Gradle, MSBuild, packaging, or project execution until the pending queue has been resolved or explicitly rejected.

Read-only operations such as status/history may run without applying patches.

## 4. One-file `.patch` standard

The universal format should be a ZIP-compatible container with the extension `.patch`. Python `zipfile`, Rust ZIP libraries, and normal archive tooling can read the container without requiring `.zip` as the filename.

Each `.patch` is one self-contained artifact. No checksum sidecar, launcher, recovery script, or loose payload directory is required.

Recommended container:

```text
Example.patch
├── PCC_PATCH_MANIFEST.json
├── payload/
│   └── <repository-relative files>
└── optional/
    └── human-readable notes
```

The manifest should contain, at minimum:

```json
{
  "schema_version": 1,
  "patch_id": "unique-id",
  "project": {
    "id": "ember",
    "remote": "https://github.com/shifty81/Ember.git"
  },
  "authority": {
    "branch": "main",
    "head": "optional exact base"
  },
  "files": [
    {
      "path": "relative/path",
      "sha256": "payload sha256",
      "preimage_sha256": "optional current-file sha256"
    }
  ],
  "remove": [],
  "post_apply": {
    "command": "gate.full",
    "handoff": "always"
  }
}
```

Patch manifests must not execute arbitrary shell, PowerShell, Python, Cargo, or other free-form commands. `post_apply.command` references a registered project command ID only.

## 5. Downloads intake

Default user-facing intake location:

`%USERPROFILE%\Downloads`

The universal PCC should scan the top level of Downloads for `*.patch`:

- on PCC startup;
- on project activation;
- before mutating operations;
- on manual Refresh/Apply Updates;
- optionally through a debounced file watcher.

The PCC reads only the manifest first. It resolves the target project through project ID plus repository authority. Unknown or ambiguous targets remain untouched and are reported.

For a recognized patch, the PCC copies it to a managed staging area and verifies the copied SHA-256 before doing anything to the project. After the patch has been safely archived with a receipt, the original Downloads copy should be removed automatically so Downloads remains clean. Failure archival must complete before removing the source file.

Do not scatter patch ZIPs, handoff ZIPs, checksum sidecars, or temporary launchers around project roots or Downloads.

Suggested managed layout:

```text
%LOCALAPPDATA%\ProjectControlCenter\
├── patches\
│   ├── staging\
│   ├── receipts\
│   ├── consumed\
│   └── failed\
└── registry\
```

Projects may additionally retain project-local transaction evidence under `artifacts/`, but the transport file itself does not need to remain at repository root.

## 6. Transaction rules

Before writing or removing any project file:

- validate project ID and repository remote;
- validate branch/HEAD requirements when declared;
- reject absolute paths, traversal, drive-qualified paths, `.git`, build outputs, and protected PCC state unless explicitly allowed for a PCC self-update;
- validate payload hashes;
- validate preimage hashes when supplied;
- snapshot every touched path;
- record whether each path existed before the transaction.

Application failure means automatic rollback.

A post-apply quality-gate failure normally **does not** roll back the source patch. It creates a FAIL handoff so the next corrective patch can be based on the exact failed state. A manifest may request gate-failure rollback only for migrations that cannot safely remain applied.

## 7. Ember FULL QUALITY GATE

The authoritative Ember full gate is ordered as follows:

```text
01  Project/root authority audit
02  Git / Cargo / rustc dependency preflight
03  cargo fmt --all
04  cargo fmt --all -- --check
05  cargo check --workspace --all-targets
06  cargo test --workspace
07  cargo clippy --workspace --all-targets -- -D warnings
08  cargo run -p ember_validation --bin ember_validate -- architecture .
09  cargo test -p ember_tools --test certification
10  cargo build -p ember_editor
11  cargo build -p ember_runtime_host
12  source fingerprint / gate evidence
13  PASS or FAIL handoff
```

The universal PCC may execute these stages itself **only when it is using the Ember project contract**. A generic Rust fallback that omits architecture validation, certification, editor/runtime targets, source fingerprinting, or handoff generation is not an Ember FULL gate.

A future native editor promotion may change the editor package. That package name must come from `project.control.json`, not from a hard-coded universal rule.

## 8. Ember fast/targeted operations

Useful project commands for the universal PCC include:

```text
project.status
gate.full
gate.fast
audit.architecture
test.certification
build.editor
build.runtime
run.editor
run.runtime
patch.status
patch.apply
git.status
git.history
git.commit-green
git.push
git.pull
git.repair-working-copy
diagnostics.bundle
package.source-rollup
```

All build/run/gate operations should enter through `PROJECT_CONTROL_CENTER.cmd` or an equivalent registered Ember provider so project-local patch intake is still honored even when the universal pre-operation host is unavailable.

## 9. Handoff contract

Every applied patch that requests certification produces one canonical handoff:

`artifacts/handoffs/Ember_Handoff_<timestamp>_<patch-id>_<PASS|FAIL>.zip`

The handoff should contain:

```text
handoff.json
HANDOFF.md
transaction/transaction.json
quality-gate/latest.json
logs/latest.log
debug/latest-debug-bundle.zip   # failure only, when produced by this lifecycle
```

The handoff must identify:

- patch ID;
- project ID;
- exact Git HEAD;
- working-tree status;
- transaction;
- quality-gate result;
- failed stage and exit code;
- latest session log;
- debug bundle associated with this lifecycle.

The GUI should reveal the handoff on failure. Cortex may consume it directly without requiring the user to upload it manually.

## 10. Cortex project creation from scratch

When Cortex creates a new project, project operations must be initialized as part of project creation rather than added after the first failure.

Required creation sequence:

```text
create project root
-> create source-control repository
-> register GitHub + Cortex-owned local Forgejo authority
-> create project.control.json
-> create PCC/project requirements metadata
-> register build/test/run/package commands
-> create patch/handoff artifact roots
-> run initial project bootstrap gate
-> capture first GREEN authority
```

After the initial bootstrap transaction, Cortex should use the same patch mechanism for automated source repairs that the user and ChatGPT use. It must not develop a second direct-overwrite repair channel.

## 11. Cortex automated repair loop

When a Cortex-created project fails a build or gate:

```text
failure
-> collect current Git authority + local diff + gate log + debug/handoff evidence
-> diagnose
-> produce exactly one project-targeted .patch
-> submit it to the universal patch intake
-> transactional apply
-> rerun the failed gate / FULL gate
-> produce handoff
-> if PASS: record GREEN and continue
-> if FAIL: consume new handoff and produce the next bounded patch
```

Cortex-generated patches use the same manifest, validation, backup, rollback, receipts, and project command IDs as externally supplied patches.

Cortex should impose a bounded autonomous repair budget. After repeated failures of the same stage/signature, stop and surface the handoff instead of looping indefinitely.

## 12. Universal PCC internal gate engine

The universal PCC can own the process host and generic stage runner. Project-specific information comes from the project contract.

The internal gate engine should provide:

- ordered stages with fail-fast behavior;
- live stdout/stderr streaming;
- exact exit codes and elapsed time;
- per-stage logs;
- environment/toolchain capture;
- cancellation;
- timeout policy;
- process-tree cleanup;
- pre/post repository hygiene;
- patch intake before gate execution;
- project-specific validators;
- project-specific certification tests;
- target builds;
- source fingerprinting;
- machine-readable evidence;
- debug bundle generation;
- handoff generation;
- GREEN eligibility state.

The universal engine should therefore execute **data-declared project steps**, not guess that every Rust project is only `fmt/check/test/clippy/build`.

## 13. Source control

Git is mandatory. GitHub is first-class remote authority and Cortex's local Forgejo service is first-class local repository/mirror/recovery authority.

A GREEN commit operation must verify that the current source fingerprint still matches the last passing FULL gate before commit/push.

Patch application and gate evidence should record the pre-patch and post-patch Git state without automatically committing.

## 14. Current Ember debugging rule

For iterative Ember work:

```text
current repository + current handoff
-> one bounded corrective patch
-> PCC apply
-> FULL gate
-> new handoff
```

Do not restart from old rollups and do not combine unrelated feature development into a compile-repair patch.

The R020 recovery has now progressed past the stale `validate_architecture` / `AssetRegistry::load` failures. The current gate reaches Foundation V2 and is blocked by the `ember_bridge` / `ember_cortex_adapter` handshake compatibility mismatch; that is the only issue addressed by the accompanying REBASE-03 source fix.
