# Ember Patch System v2

Ember patches are repository-root drop-in ZIPs. `PROJECT_CONTROL_CENTER.cmd` now starts through a stable bootstrap that processes pending updates before **every** PCC operation, including FULL QUALITY GATE, build, run, and interactive startup.

## Required lifecycle

A normal update is:

1. Drop the ZIP into the Ember repository root.
2. Start the PCC or run any PCC build/gate command.
3. The bootstrap discovers root-drop patches before pending recovery plans and before the requested operation.
4. The patch is authority-checked, checksum-verified, backed up and applied transactionally.
5. If PCC tooling itself changed, the Python process re-execs so post-apply work uses the newly installed tooling.
6. The patch's allow-listed `post_apply.run` action executes.
7. A PASS/FAIL handoff ZIP is written under `artifacts/handoffs/` and revealed in Explorer when requested.
8. Failed apply transactions roll back. Gate failures retain the applied source plus recovery backup and produce a debug/handoff bundle for diagnosis.

No patch manifest may execute arbitrary shell, PowerShell, Python, Cargo, or other free-form commands. Post-apply operations are selected from the PCC's fixed allowlist.

## Schema v2 additions

Schema v1 `files` and `remove` remain supported. Schema v2 adds:

- `authority.remote`, `authority.branch`, and `authority.head` preconditions.
- `reconcile` operations for repair/rebase patches. A tracked path is restored from the declared Git authority.
  For an untracked exact path, `untracked_policy` controls recovery:
  - `match_hash`: remove only when `unexpected_sha256` matches.
  - `backup_and_remove`: back up the current bytes transactionally, then remove even if a formatter/generator changed them.
  - `preserve`: record and leave the untracked path untouched.
  If `untracked_policy` is omitted, entries with `unexpected_sha256` behave as `match_hash`; entries without it behave as `backup_and_remove` for backward compatibility.
- `post_apply.run`: `none`, `full_gate`, `cargo_check`, `build_editor`, `build_runtime`, or `certification`.
- `post_apply.handoff`: `none`, `always`, `on_success`, or `on_failure`.
- `post_apply.open_handoff`: reveal the handoff ZIP after lifecycle completion.
- automatic PCC self-reload when the patch changes control-center tooling.

## Handoff authority

`artifacts/handoffs/Ember_Handoff_<timestamp>_<patch>_<PASS|FAIL>.zip` is the upload artifact for the next development pass. It includes:

- `handoff.json`
- `HANDOFF.md`
- the patch transaction record
- latest quality-gate evidence
- latest PCC session log
- latest debug bundle when the lifecycle failed
- Git HEAD and working-tree status

`artifacts/handoffs/LATEST_HANDOFF.txt` points to the latest handoff.

## Recovery patches

Repairs must use the same patch engine. Do not ship stand-alone recovery launchers when the PCC intake path is functional. Root-drop repair patches have priority over pending plans so a defective plan cannot block its own repair. A repair patch should declare the exact Git authority and reconcile only the paths implicated by the faulty patch. The transaction backup remains available under `artifacts/recovery/`.

## One-time v1-to-v2 bootstrap

The pre-v2 PCC loads its Python module before applying root-drop updates, so the patch that installs this bootstrap cannot hot-reload that already-running legacy process. For this migration only, let the existing PCC apply the bootstrap patch, then close and launch `PROJECT_CONTROL_CENTER.cmd` once more. The pending recovery plan is then executed automatically before any normal operation. After this bootstrap is installed, control-center self-updates use process re-exec and no further manual restart is required.

## Recovery safety rule

Recovery transactions always back up every exact path before mutation. A `backup_and_remove`
operation is valid only for a manifest-listed exact path known to have been introduced by the
faulty patch; it is not a directory wipe or wildcard delete. Changed bytes remain recoverable
under `artifacts/recovery/<transaction>/files/`.

Handoffs only attach quality-gate, log, and debug artifacts created after the current patch
transaction, preventing stale evidence from being mislabeled as part of a new recovery.
