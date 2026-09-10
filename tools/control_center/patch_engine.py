from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any, Callable, Iterable

PROJECT_ID = "ember"
EXPECTED_REMOTE = "https://github.com/shifty81/Ember.git"
DEFAULT_BRANCH = "main"
PATCH_MANIFEST_NAMES = (
    "EMBER_PATCH_MANIFEST.json",
    "ember-patch.json",
    "patch-manifest.json",
)
PROTECTED_ROOTS = {".git", "artifacts", "target"}
RELOAD_PATHS = {
    "PROJECT_CONTROL_CENTER.cmd",
    "tools/control_center/pcc_bootstrap.py",
    "tools/control_center/patch_engine.py",
    "tools/control_center/ember_pcc.py",
}
ALLOWED_POST_ACTIONS = {
    "none",
    "full_gate",
    "cargo_check",
    "build_editor",
    "build_runtime",
    "certification",
}


def now_local() -> dt.datetime:
    return dt.datetime.now().astimezone()


def stamp() -> str:
    return now_local().strftime("%Y%m%d-%H%M%S")


def iso_now() -> str:
    return now_local().isoformat(timespec="seconds")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def normalize_remote(value: str) -> str:
    text = value.strip().replace("\\", "/")
    if text.endswith(".git"):
        text = text[:-4]
    if text.startswith("git@github.com:"):
        text = "https://github.com/" + text[len("git@github.com:") :]
    return text.rstrip("/").lower()


def safe_rel(value: str) -> Path:
    pure = PurePosixPath(value.replace("\\", "/"))
    if pure.is_absolute() or not pure.parts or any(part in ("", ".", "..") for part in pure.parts):
        raise ValueError(f"Unsafe patch path: {value}")
    if pure.parts[0].lower() in PROTECTED_ROOTS:
        raise ValueError(f"Patch may not modify protected path: {value}")
    if ":" in pure.parts[0]:
        raise ValueError(f"Unsafe drive-qualified patch path: {value}")
    return Path(*pure.parts)


@dataclass
class PatchResult:
    ok: bool
    patch_id: str
    transaction_path: Path | None = None
    archived_patch: Path | None = None
    restart_required: bool = False
    lifecycle_path: Path | None = None
    error: str | None = None


class PatchEngine:
    """Project-local transactional patch engine shared by PCC startup and menus.

    Schema v1 remains readable for backward compatibility. Schema v2 adds:
      * Git authority preconditions
      * reconcile-to-authority operations for repair patches
      * post-apply project actions from a strict allowlist
      * upload-ready handoff bundles
      * self-reload state when PCC tooling is updated
      * explicit safe handling for changed untracked recovery files

    No arbitrary script execution is allowed by the patch manifest.
    """

    def __init__(
        self,
        root: Path,
        *,
        status: Callable[[str, str], None] | None = None,
        log: Callable[[str, str], None] | None = None,
        interactive: bool = True,
    ) -> None:
        self.root = root.resolve()
        self.status_cb = status
        self.log_cb = log
        self.interactive = interactive
        self.artifacts = self.root / "artifacts"
        self.update_root = self.artifacts / "updates"
        self.consumed = self.update_root / "consumed"
        self.failed = self.update_root / "failed"
        self.recovery = self.artifacts / "recovery"
        self.handoffs = self.artifacts / "handoffs"
        self.logs = self.artifacts / "logs" / "sessions"
        self.gates = self.artifacts / "quality-gates"
        self.debug_bundles = self.artifacts / "debug-bundles"
        self.state_root = self.root / ".ember" / "pcc"
        for directory in (
            self.consumed,
            self.failed,
            self.recovery,
            self.handoffs,
            self.state_root,
        ):
            directory.mkdir(parents=True, exist_ok=True)

    def status(self, label: str, message: str) -> None:
        if self.status_cb:
            self.status_cb(label, message)
        else:
            print(f"[{label}] {message}")
        self.log(message, label)

    def log(self, message: str, level: str = "INFO") -> None:
        if self.log_cb:
            try:
                self.log_cb(message, level)
            except TypeError:
                self.log_cb(message)  # type: ignore[misc]

    def run(self, args: list[str], *, check: bool = False) -> subprocess.CompletedProcess[bytes]:
        result = subprocess.run(
            args,
            cwd=self.root,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        if check and result.returncode != 0:
            text = result.stdout.decode("utf-8", errors="replace")
            raise RuntimeError(f"command failed ({result.returncode}): {' '.join(args)}\n{text}")
        return result

    def _git_text(self, args: list[str], *, check: bool = True) -> str:
        result = self.run(["git", *args], check=check)
        return result.stdout.decode("utf-8", errors="replace").strip()

    def _validate_checksum(self, value: Any, label: str) -> str:
        checksum = str(value or "").lower()
        if len(checksum) != 64 or any(ch not in "0123456789abcdef" for ch in checksum):
            raise ValueError(f"Missing/invalid sha256 for {label}")
        return checksum

    def inspect_patch(self, zip_path: Path) -> tuple[dict[str, Any], str] | None:
        try:
            with zipfile.ZipFile(zip_path) as archive:
                names = set(archive.namelist())
                manifest_name = next((name for name in PATCH_MANIFEST_NAMES if name in names), None)
                if not manifest_name:
                    return None
                manifest = json.loads(archive.read(manifest_name).decode("utf-8"))
                schema = int(manifest.get("schema_version", 0))
                if schema not in (1, 2):
                    raise ValueError(f"Unsupported patch schema_version {schema}")
                if manifest.get("project_id") != PROJECT_ID:
                    raise ValueError(f"Patch targets {manifest.get('project_id')!r}, not '{PROJECT_ID}'")
                patch_id = str(manifest.get("patch_id") or zip_path.stem).strip()
                if not patch_id:
                    raise ValueError("Patch id is empty")
                if len(patch_id) > 120 or any(not (ch.isalnum() or ch in "._-") for ch in patch_id):
                    raise ValueError("Patch id may contain only letters, digits, dot, underscore and hyphen")

                files = manifest.get("files", [])
                if not isinstance(files, list):
                    raise ValueError("files must be an array")
                if schema == 1 and not files:
                    raise ValueError("Schema v1 patch requires a non-empty files array")
                seen: set[str] = set()
                for entry in files:
                    if not isinstance(entry, dict):
                        raise ValueError("Each files entry must be an object")
                    rel = safe_rel(str(entry.get("path", ""))).as_posix()
                    if rel in seen:
                        raise ValueError(f"Duplicate patch path: {rel}")
                    seen.add(rel)
                    checksum = self._validate_checksum(entry.get("sha256"), rel)
                    member = f"payload/{rel}"
                    if member not in names:
                        raise ValueError(f"Missing payload member: {member}")
                    if sha256_bytes(archive.read(member)) != checksum:
                        raise ValueError(f"Checksum mismatch for {rel}")

                removes = manifest.get("remove", [])
                if not isinstance(removes, list):
                    raise ValueError("remove must be an array")
                for raw in removes:
                    rel = safe_rel(str(raw)).as_posix()
                    if rel in seen:
                        raise ValueError(f"Path cannot be both written and removed: {rel}")
                    seen.add(rel)

                reconcile = manifest.get("reconcile", [])
                if schema == 1 and reconcile:
                    raise ValueError("reconcile requires patch schema v2")
                if not isinstance(reconcile, list):
                    raise ValueError("reconcile must be an array")
                for entry in reconcile:
                    if not isinstance(entry, dict):
                        raise ValueError("Each reconcile entry must be an object")
                    rel = safe_rel(str(entry.get("path", ""))).as_posix()
                    if rel in seen:
                        raise ValueError(f"Duplicate patch operation path: {rel}")
                    seen.add(rel)
                    bad = entry.get("unexpected_sha256")
                    if bad is not None:
                        self._validate_checksum(bad, f"reconcile {rel}")
                    policy = entry.get("untracked_policy")
                    if policy is not None and str(policy) not in {
                        "match_hash",
                        "backup_and_remove",
                        "preserve",
                    }:
                        raise ValueError(
                            f"Unsupported reconcile untracked_policy for {rel}: {policy}"
                        )
                    if str(policy) == "match_hash" and bad is None:
                        raise ValueError(
                            f"reconcile {rel} uses match_hash but has no unexpected_sha256"
                        )

                if not files and not removes and not reconcile:
                    raise ValueError("Patch has no operations")

                post = manifest.get("post_apply", {})
                if post is None:
                    post = {}
                if not isinstance(post, dict):
                    raise ValueError("post_apply must be an object")
                action = str(post.get("run", "none"))
                if action not in ALLOWED_POST_ACTIONS:
                    raise ValueError(f"Unsupported post_apply.run: {action}")
                handoff = str(post.get("handoff", "none"))
                if handoff not in {"none", "always", "on_failure", "on_success"}:
                    raise ValueError(f"Unsupported post_apply.handoff: {handoff}")

                authority = manifest.get("authority", {})
                if authority is None:
                    authority = {}
                if not isinstance(authority, dict):
                    raise ValueError("authority must be an object")
                if reconcile and not authority.get("head"):
                    raise ValueError("reconcile patches require authority.head")
                if reconcile:
                    head = str(authority.get("head", ""))
                    if len(head) != 40 or any(ch not in "0123456789abcdefABCDEF" for ch in head):
                        raise ValueError("reconcile patches require an exact 40-hex authority.head")
                return manifest, patch_id
        except Exception as exc:
            raise ValueError(f"Invalid patch {zip_path.name}: {exc}") from exc

    def scan_root_patches(self) -> list[Path]:
        patches: list[Path] = []
        for path in sorted(self.root.glob("*.zip")):
            try:
                if self.inspect_patch(path):
                    patches.append(path)
            except Exception as exc:
                self.log(str(exc), "WARN")
                patches.append(path)
        return patches

    def _check_authority(self, manifest: dict[str, Any]) -> None:
        authority = manifest.get("authority") or {}
        if not authority:
            return
        if not (self.root / ".git").exists():
            raise RuntimeError("Patch requires Git authority but repository has no .git directory")

        required_head = str(authority.get("head") or "").strip()
        if required_head:
            actual = self._git_text(["rev-parse", "HEAD"])
            if actual.lower() != required_head.lower():
                raise RuntimeError(f"Patch authority HEAD mismatch: expected {required_head}, actual {actual}")

        required_branch = str(authority.get("branch") or "").strip()
        if required_branch:
            actual = self._git_text(["branch", "--show-current"])
            if actual != required_branch:
                raise RuntimeError(f"Patch authority branch mismatch: expected {required_branch}, actual {actual}")

        required_remote = str(authority.get("remote") or "").strip()
        if required_remote:
            actual = self._git_text(["remote", "get-url", "origin"])
            if normalize_remote(actual) != normalize_remote(required_remote):
                raise RuntimeError(f"Patch authority remote mismatch: expected {required_remote}, actual {actual}")

    def _is_tracked_at(self, source: str, rel: Path) -> bool:
        spec = f"{source}:{rel.as_posix()}"
        return self.run(["git", "cat-file", "-e", spec]).returncode == 0

    def _git_blob(self, source: str, rel: Path) -> bytes:
        result = self.run(["git", "show", f"{source}:{rel.as_posix()}"], check=True)
        return result.stdout

    def _backup_paths(self, paths: Iterable[Path], backup_root: Path, state: dict[str, Any]) -> None:
        backup_files = backup_root / "files"
        for rel in paths:
            key = rel.as_posix()
            target = self.root / rel
            if target.exists() and target.is_file():
                backup = backup_files / rel
                backup.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(target, backup)
                state["previous"][key] = {
                    "exists": True,
                    "sha256": sha256_file(target),
                }
            elif not target.exists():
                state["previous"][key] = {"exists": False}
            else:
                raise RuntimeError(f"Patch target is not a regular file: {rel}")

    def _atomic_write(self, target: Path, data: bytes) -> None:
        target.parent.mkdir(parents=True, exist_ok=True)
        temp = target.with_name(target.name + ".ember-patch.tmp")
        temp.write_bytes(data)
        os.replace(temp, target)

    def _rollback(self, backup_root: Path) -> None:
        tx_path = backup_root / "transaction.json"
        if not tx_path.exists():
            return
        state = json.loads(tx_path.read_text(encoding="utf-8"))
        for rel_text, previous in state.get("previous", {}).items():
            rel = safe_rel(rel_text)
            target = self.root / rel
            existed = bool(previous.get("exists")) if isinstance(previous, dict) else bool(previous)
            if existed:
                backup = backup_root / "files" / rel
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(backup, target)
            elif target.exists() and target.is_file():
                target.unlink()
        state["status"] = "ROLLED_BACK"
        state["rolled_back_at"] = iso_now()
        tx_path.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")
        self.log(f"PATCH ROLLBACK PASS transaction={backup_root.name}")

    def _archive_failed(self, zip_path: Path, reason: str) -> None:
        if not zip_path.exists():
            return
        target = self.failed / f"{zip_path.stem}_{stamp()}{zip_path.suffix}"
        shutil.move(str(zip_path), target)
        target.with_suffix(target.suffix + ".error.txt").write_text(reason + "\n", encoding="utf-8")

    def _create_lifecycle(self, patch_id: str, transaction_path: Path, post: dict[str, Any]) -> Path | None:
        action = str(post.get("run", "none"))
        handoff = str(post.get("handoff", "none"))
        if action == "none" and handoff == "none":
            return None
        record = {
            "schema_version": 1,
            "project_id": PROJECT_ID,
            "patch_id": patch_id,
            "created_at": iso_now(),
            "transaction_path": str(transaction_path),
            "post_apply": {
                "run": action,
                "handoff": handoff,
                "open_handoff": bool(post.get("open_handoff", True)),
            },
        }
        path = self.state_root / f"pending-lifecycle-{stamp()}-{patch_id}.json"
        path.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        return path

    def apply_patch(self, zip_path: Path) -> PatchResult:
        try:
            inspected = self.inspect_patch(zip_path)
            if not inspected:
                return PatchResult(False, zip_path.stem, error="Not an Ember patch")
            manifest, patch_id = inspected
            return self._apply_manifest(manifest, patch_id, zip_path=zip_path)
        except Exception as exc:
            reason = str(exc)
            self.status("FAIL", reason)
            self._archive_failed(zip_path, reason)
            return PatchResult(False, zip_path.stem, error=reason)

    def apply_plan_file(self, plan_path: Path) -> PatchResult:
        patch_id = plan_path.stem
        try:
            manifest = json.loads(plan_path.read_text(encoding="utf-8"))
            if int(manifest.get("schema_version", 0)) != 2 or manifest.get("project_id") != PROJECT_ID:
                raise ValueError(f"Invalid pending Ember patch plan: {plan_path}")
            patch_id = str(manifest.get("patch_id") or plan_path.stem)
            if len(patch_id) > 120 or any(not (ch.isalnum() or ch in "._-") for ch in patch_id):
                raise ValueError("Patch id may contain only letters, digits, dot, underscore and hyphen")
            result = self._apply_manifest(manifest, patch_id, zip_path=None)
        except Exception as exc:
            result = PatchResult(False, patch_id, error=str(exc))

        if plan_path.exists():
            destination_root = self.consumed if result.ok else self.failed
            archived = destination_root / f"{plan_path.stem}_{stamp()}.json"
            shutil.move(str(plan_path), archived)
            if not result.ok:
                archived.with_suffix(archived.suffix + ".error.txt").write_text(
                    (result.error or "pending plan failed") + "\n", encoding="utf-8"
                )
        return result

    def _apply_manifest(
        self,
        manifest: dict[str, Any],
        patch_id: str,
        *,
        zip_path: Path | None,
    ) -> PatchResult:
        self._check_authority(manifest)
        schema = int(manifest.get("schema_version", 0))
        files = manifest.get("files", [])
        removes = [safe_rel(str(value)) for value in manifest.get("remove", [])]
        reconcile = manifest.get("reconcile", []) if schema >= 2 else []
        authority = manifest.get("authority") or {}
        authority_head = str(authority.get("head") or "HEAD")
        post = manifest.get("post_apply") or {}

        writes: list[tuple[Path, bytes, str]] = []
        if files:
            if zip_path is None:
                raise ValueError("Pending plan may not contain payload files")
            with zipfile.ZipFile(zip_path) as archive:
                for entry in files:
                    rel = safe_rel(str(entry["path"]))
                    expected = self._validate_checksum(entry.get("sha256"), rel.as_posix())
                    data = archive.read(f"payload/{rel.as_posix()}")
                    writes.append((rel, data, expected))

        reconcile_ops: list[tuple[Path, str | None, str]] = []
        operation_paths = {rel.as_posix() for rel, _, _ in writes}
        operation_paths.update(rel.as_posix() for rel in removes)
        for entry in reconcile:
            if not isinstance(entry, dict):
                raise ValueError("Each reconcile entry must be an object")
            rel = safe_rel(str(entry["path"]))
            rel_text = rel.as_posix()
            if rel_text in operation_paths:
                raise ValueError(f"Duplicate patch operation path: {rel_text}")
            operation_paths.add(rel_text)
            bad = entry.get("unexpected_sha256")
            bad_sha = (
                self._validate_checksum(bad, f"reconcile {rel_text}")
                if bad is not None
                else None
            )
            raw_policy = entry.get("untracked_policy")
            if raw_policy is None:
                policy = "match_hash" if bad_sha is not None else "backup_and_remove"
            else:
                policy = str(raw_policy)
            if policy not in {"match_hash", "backup_and_remove", "preserve"}:
                raise ValueError(
                    f"Unsupported reconcile untracked_policy for {rel_text}: {policy}"
                )
            if policy == "match_hash" and bad_sha is None:
                raise ValueError(
                    f"reconcile {rel_text} uses match_hash but has no unexpected_sha256"
                )
            reconcile_ops.append((rel, bad_sha, policy))

        touched = [rel for rel, _, _ in writes] + removes + [rel for rel, _, _ in reconcile_ops]
        tx_id = f"{stamp()}-{patch_id}"
        backup_root = self.recovery / tx_id
        backup_root.mkdir(parents=True, exist_ok=True)
        tx_path = backup_root / "transaction.json"
        state: dict[str, Any] = {
            "schema_version": 2,
            "project_id": PROJECT_ID,
            "patch_id": patch_id,
            "source_package": zip_path.name if zip_path else None,
            "created_at": iso_now(),
            "authority": authority,
            "previous": {},
            "operations": [],
            "status": "APPLYING",
        }
        self._backup_paths(touched, backup_root, state)
        tx_path.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")

        self.log(f"PATCH APPLY START patch={patch_id}")
        try:
            for rel, data, expected in writes:
                self._atomic_write(self.root / rel, data)
                state["operations"].append({"kind": "write", "path": rel.as_posix(), "sha256": expected})

            for rel in removes:
                target = self.root / rel
                if target.exists():
                    target.unlink()
                state["operations"].append({"kind": "remove", "path": rel.as_posix()})

            for rel, bad_sha, policy in reconcile_ops:
                target = self.root / rel
                if self._is_tracked_at(authority_head, rel):
                    data = self._git_blob(authority_head, rel)
                    self._atomic_write(target, data)
                    state["operations"].append(
                        {
                            "kind": "reconcile_restore",
                            "path": rel.as_posix(),
                            "source": authority_head,
                            "sha256": sha256_bytes(data),
                        }
                    )
                elif target.exists():
                    if not target.is_file():
                        raise RuntimeError(f"Untracked reconcile target is not a regular file: {rel}")
                    current = sha256_file(target)
                    if policy == "preserve":
                        state["operations"].append(
                            {
                                "kind": "reconcile_preserve_untracked",
                                "path": rel.as_posix(),
                                "sha256": current,
                            }
                        )
                        continue
                    if policy == "match_hash" and current != bad_sha:
                        raise RuntimeError(
                            f"Refusing to remove changed untracked file {rel}: expected stale hash {bad_sha}, current {current}"
                        )
                    # All touched paths were backed up before mutation. The
                    # backup_and_remove policy is therefore safe for an exact
                    # path known to have been introduced by a faulty patch even
                    # when formatters or generators changed its bytes later.
                    target.unlink()
                    state["operations"].append(
                        {
                            "kind": "reconcile_remove_untracked",
                            "path": rel.as_posix(),
                            "removed_sha256": current,
                            "untracked_policy": policy,
                            "backup": str((backup_root / "files" / rel).resolve()),
                        }
                    )
                else:
                    state["operations"].append({"kind": "reconcile_noop_missing", "path": rel.as_posix()})

            for rel, _, expected in writes:
                target = self.root / rel
                if not target.is_file() or sha256_file(target) != expected:
                    raise RuntimeError(f"Post-apply verification failed: {rel}")
            for rel in removes:
                if (self.root / rel).exists():
                    raise RuntimeError(f"Post-apply removal failed: {rel}")
            for rel, _, policy in reconcile_ops:
                target = self.root / rel
                if self._is_tracked_at(authority_head, rel):
                    expected = sha256_bytes(self._git_blob(authority_head, rel))
                    if not target.is_file() or sha256_file(target) != expected:
                        raise RuntimeError(f"Post-reconcile verification failed: {rel}")
                elif policy == "preserve":
                    # Preservation is intentional; backup evidence still records
                    # the exact pre-transaction state.
                    continue
                elif target.exists():
                    raise RuntimeError(f"Post-reconcile untracked removal failed: {rel}")

            state["status"] = "APPLIED"
            state["finished_at"] = iso_now()
            tx_path.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")

            archived_patch: Path | None = None
            if zip_path is not None and zip_path.exists():
                archived_patch = self.consumed / f"{zip_path.stem}_{stamp()}{zip_path.suffix}"
                shutil.move(str(zip_path), archived_patch)

            lifecycle = self._create_lifecycle(patch_id, tx_path, post)
            touched_text = {rel.as_posix() for rel in touched}
            restart_required = bool(manifest.get("restart_pcc", False)) or any(
                path in RELOAD_PATHS or path.startswith("tools/control_center/") for path in touched_text
            )
            self.status("PASS", f"Applied patch {patch_id} transactionally")
            self.log(f"PATCH APPLY PASS patch={patch_id}")
            return PatchResult(
                True,
                patch_id,
                transaction_path=tx_path,
                archived_patch=archived_patch,
                restart_required=restart_required,
                lifecycle_path=lifecycle,
            )
        except Exception as exc:
            reason = str(exc)
            self.log(f"PATCH APPLY FAIL patch={patch_id} error={reason}", "ERROR")
            self._rollback(backup_root)
            if zip_path is not None:
                self._archive_failed(zip_path, reason)
            self.status("FAIL", f"Patch {patch_id} failed and was rolled back: {reason}")
            return PatchResult(False, patch_id, transaction_path=tx_path, error=reason)

    def process_root_patches(self) -> list[PatchResult]:
        results: list[PatchResult] = []
        for path in self.scan_root_patches():
            result = self.apply_patch(path)
            results.append(result)
            if not result.ok or result.restart_required:
                break
        return results

    def pending_plan_paths(self) -> list[Path]:
        paths = []
        canonical = self.state_root / "pending_recovery.json"
        if canonical.exists():
            paths.append(canonical)
        paths.extend(sorted(self.state_root.glob("pending-plan-*.json")))
        return paths

    def pending_lifecycle_paths(self) -> list[Path]:
        return sorted(self.state_root.glob("pending-lifecycle-*.json"))

    def reveal_file(self, path: Path) -> None:
        if not self.interactive:
            return
        try:
            if os.name == "nt":
                subprocess.Popen(["explorer.exe", f"/select,{path}"])
            elif sys.platform == "darwin":
                subprocess.Popen(["open", "-R", str(path)])
            else:
                subprocess.Popen(["xdg-open", str(path.parent)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception as exc:
            self.log(f"Could not reveal handoff {path}: {exc}", "WARN")

    def _latest(self, root: Path, pattern: str) -> Path | None:
        candidates = sorted(root.glob(pattern), key=lambda p: p.stat().st_mtime, reverse=True)
        return candidates[0] if candidates else None

    def create_handoff(
        self,
        *,
        patch_id: str,
        status: str,
        reason: str,
        transaction_path: Path | None,
        open_handoff: bool,
    ) -> Path:
        token = stamp()
        output = self.handoffs / f"Ember_Handoff_{token}_{patch_id}_{status}.zip"
        cutoff = transaction_path.stat().st_mtime if transaction_path and transaction_path.exists() else 0.0
        latest_gate = self._latest(self.gates, "*.json")
        latest_log = self._latest(self.logs, "*.log")
        latest_debug = self._latest(self.debug_bundles, "*.zip") if status != "PASS" else None
        # A handoff must never imply that stale evidence belongs to the current
        # patch lifecycle. Only attach artifacts created after this transaction.
        if latest_gate is not None and latest_gate.stat().st_mtime < cutoff:
            latest_gate = None
        if latest_log is not None and latest_log.stat().st_mtime < cutoff:
            latest_log = None
        if latest_debug is not None and latest_debug.stat().st_mtime < cutoff:
            latest_debug = None
        git_status = self._git_text(["status", "--porcelain=v1"], check=False) if (self.root / ".git").exists() else ""
        git_head = self._git_text(["rev-parse", "HEAD"], check=False) if (self.root / ".git").exists() else ""
        summary = {
            "schema_version": 1,
            "project_id": PROJECT_ID,
            "patch_id": patch_id,
            "status": status,
            "reason": reason,
            "created_at": iso_now(),
            "git_head": git_head,
            "git_status": git_status.splitlines(),
            "transaction": str(transaction_path) if transaction_path else None,
            "quality_gate": str(latest_gate) if latest_gate else None,
            "debug_bundle": str(latest_debug) if latest_debug else None,
            "session_log": str(latest_log) if latest_log else None,
        }
        markdown = [
            "# Ember Patch Handoff",
            "",
            f"- Patch: `{patch_id}`",
            f"- Status: **{status}**",
            f"- Reason: {reason}",
            f"- Git HEAD: `{git_head or 'unavailable'}`",
            f"- Created: {summary['created_at']}",
            "",
            "Upload this handoff ZIP into the Ember development chat. It contains the patch transaction, gate evidence, latest session log, Git status, and the latest debug bundle when the lifecycle failed.",
            "",
        ]
        with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
            archive.writestr("handoff.json", json.dumps(summary, indent=2) + "\n")
            archive.writestr("HANDOFF.md", "\n".join(markdown))
            for label, path in (
                ("transaction/transaction.json", transaction_path),
                ("quality-gate/latest.json", latest_gate),
                ("logs/latest.log", latest_log),
                ("debug/latest-debug-bundle.zip", latest_debug),
            ):
                if path and path.is_file():
                    archive.write(path, label)
        (self.handoffs / "LATEST_HANDOFF.txt").write_text(str(output) + "\n", encoding="utf-8")
        self.status("PASS" if status == "PASS" else "WARN", f"Handoff: {output}")
        if open_handoff:
            self.reveal_file(output)
        return output

    def run_lifecycle(self, host: Any, lifecycle_path: Path) -> bool:
        record = json.loads(lifecycle_path.read_text(encoding="utf-8"))
        patch_id = str(record.get("patch_id") or lifecycle_path.stem)
        post = record.get("post_apply") or {}
        action = str(post.get("run", "none"))
        handoff_mode = str(post.get("handoff", "none"))
        open_handoff = bool(post.get("open_handoff", True))
        transaction_text = record.get("transaction_path")
        transaction_path = Path(transaction_text) if transaction_text else None

        ok = True
        reason = "Patch applied"
        previous_noninteractive = getattr(host, "noninteractive", None)
        if previous_noninteractive is not None:
            # Patch lifecycles reveal one canonical handoff, not several competing
            # debug/artifact folders from lower-level operations.
            host.noninteractive = True
        try:
            if action == "full_gate":
                ok = bool(host.full_gate())
                reason = "FULL QUALITY GATE passed" if ok else "FULL QUALITY GATE failed"
            elif action == "cargo_check":
                result = host.run(["cargo", "check", "--workspace", "--all-targets"])
                ok = result.returncode == 0
                reason = "cargo check passed" if ok else "cargo check failed"
            elif action == "build_editor":
                ok = bool(host.build_target("ember_editor"))
                reason = "editor build passed" if ok else "editor build failed"
            elif action == "build_runtime":
                ok = bool(host.build_target("ember_runtime_host"))
                reason = "runtime build passed" if ok else "runtime build failed"
            elif action == "certification":
                result = host.run(["cargo", "test", "-p", "ember_tools", "--test", "certification"])
                ok = result.returncode == 0
                reason = "certification passed" if ok else "certification failed"
        finally:
            if previous_noninteractive is not None:
                host.noninteractive = previous_noninteractive

        if not ok and action != "full_gate" and hasattr(host, "create_debug_bundle"):
            try:
                host.create_debug_bundle(reason, open_folder=False)
            except Exception as exc:
                self.log(f"Could not create lifecycle debug bundle: {exc}", "WARN")

        should_handoff = (
            handoff_mode == "always"
            or (handoff_mode == "on_success" and ok)
            or (handoff_mode == "on_failure" and not ok)
        )
        if should_handoff:
            self.create_handoff(
                patch_id=patch_id,
                status="PASS" if ok else "FAIL",
                reason=reason,
                transaction_path=transaction_path,
                open_handoff=open_handoff,
            )

        completed = self.consumed / f"{lifecycle_path.stem}_{stamp()}.json"
        payload = dict(record)
        payload["completed_at"] = iso_now()
        payload["status"] = "PASS" if ok else "FAIL"
        payload["reason"] = reason
        completed.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
        lifecycle_path.unlink(missing_ok=True)
        return ok
