#!/usr/bin/env python3
"""Ember project-local Project Control Center.

Self-contained project operations layer. Uses only the Python standard library and
project-native tools (Git/Cargo). It intentionally does not depend on Cortex,
Forge, Open2D, or an external Project Control Center runtime.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import textwrap
import time
import traceback
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Iterable, Sequence

PROJECT_ID = "ember"
PROJECT_NAME = "Ember"
EXPECTED_REMOTE = "https://github.com/shifty81/Ember.git"
DEFAULT_BRANCH = "main"
PATCH_MANIFEST_NAMES = (
    "EMBER_PATCH_MANIFEST.json",
    "ember-patch.json",
    "patch-manifest.json",
)
EXCLUDED_ROOTS = {".git", "target", "artifacts", "logs", ".ember"}
REQUIRED_ROOT = (
    "Cargo.toml",
    "README.md",
    "forge.project.json",
    "PROJECT_CONTROL_CENTER.cmd",
    "tools/control_center/ember_pcc.py",
    "apps/ember_editor/Cargo.toml",
    "apps/ember_runtime_host/Cargo.toml",
    "config/ember/capabilities.json",
    "crates/ember_packages/Cargo.toml",
    "crates/ember_jobs/Cargo.toml",
    "crates/ember_session/Cargo.toml",
    "certification/ember_smoke/ember.project.json",
)


def now_local() -> dt.datetime:
    return dt.datetime.now().astimezone()


def stamp() -> str:
    return now_local().strftime("%Y%m%d-%H%M%S")


def iso_now() -> str:
    return now_local().isoformat(timespec="seconds")


def human_size(value: int) -> str:
    units = ["B", "KB", "MB", "GB"]
    size = float(value)
    for unit in units:
        if size < 1024 or unit == units[-1]:
            return f"{size:.1f} {unit}" if unit != "B" else f"{int(size)} B"
        size /= 1024
    return f"{value} B"


def enable_windows_ansi() -> None:
    if os.name != "nt":
        return
    try:
        import ctypes

        kernel32 = ctypes.windll.kernel32
        handle = kernel32.GetStdHandle(-11)
        mode = ctypes.c_uint32()
        if kernel32.GetConsoleMode(handle, ctypes.byref(mode)):
            kernel32.SetConsoleMode(handle, mode.value | 0x0004)
    except Exception:
        pass


enable_windows_ansi()


class C:
    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    RED = "\033[91m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    CYAN = "\033[96m"


@dataclass
class CommandResult:
    returncode: int
    output: str
    elapsed: float


class PCC:
    def __init__(self, root: Path, *, noninteractive: bool = False) -> None:
        self.root = root.resolve()
        self.noninteractive = noninteractive
        self.artifacts = self.root / "artifacts"
        self.logs = self.artifacts / "logs" / "sessions"
        self.gates = self.artifacts / "quality-gates"
        self.debug_bundles = self.artifacts / "debug-bundles"
        self.snapshots = self.artifacts / "source-snapshots"
        self.update_root = self.artifacts / "updates"
        self.update_consumed = self.update_root / "consumed"
        self.update_failed = self.update_root / "failed"
        self.recovery = self.artifacts / "recovery"
        for directory in (
            self.logs,
            self.gates,
            self.debug_bundles,
            self.snapshots,
            self.update_consumed,
            self.update_failed,
            self.recovery,
        ):
            directory.mkdir(parents=True, exist_ok=True)
        self.log_path = self.logs / f"ember-pcc-{stamp()}.log"
        self._log_handle = self.log_path.open("a", encoding="utf-8", errors="replace")
        self.log(f"PCC START root={self.root}")

    def close(self) -> None:
        try:
            self.log("PCC END")
            self._log_handle.close()
        except Exception:
            pass

    def log(self, message: str, level: str = "INFO") -> None:
        line = f"[{iso_now()}] [{level}] {message}"
        self._log_handle.write(line + "\n")
        self._log_handle.flush()

    def print_status(self, label: str, message: str) -> None:
        color = C.GREEN if label == "PASS" else C.RED if label == "FAIL" else C.YELLOW
        print(f"{color}[{label}]{C.RESET} {message}")
        self.log(message, label)

    def run(
        self,
        args: Sequence[str],
        *,
        cwd: Path | None = None,
        check: bool = False,
        live: bool = True,
        env: dict[str, str] | None = None,
    ) -> CommandResult:
        cwd = cwd or self.root
        cmd_text = subprocess.list2cmdline(list(args))
        self.log(f"RUN cwd={cwd} cmd={cmd_text}")
        started = time.monotonic()
        process_env = os.environ.copy()
        if env:
            process_env.update(env)
        try:
            proc = subprocess.Popen(
                list(args),
                cwd=str(cwd),
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                encoding="utf-8",
                errors="replace",
                env=process_env,
            )
        except FileNotFoundError:
            elapsed = time.monotonic() - started
            result = CommandResult(127, f"Command not found: {args[0]}\n", elapsed)
            self.log(result.output.strip(), "ERROR")
            if check:
                raise RuntimeError(result.output.strip())
            return result
        lines: list[str] = []
        assert proc.stdout is not None
        for line in proc.stdout:
            lines.append(line)
            self._log_handle.write(line)
            self._log_handle.flush()
            if live:
                print(line, end="")
        rc = proc.wait()
        elapsed = time.monotonic() - started
        output = "".join(lines)
        self.log(f"EXIT rc={rc} elapsed={elapsed:.2f}s cmd={cmd_text}")
        result = CommandResult(rc, output, elapsed)
        if check and rc != 0:
            raise RuntimeError(f"Command failed ({rc}): {cmd_text}")
        return result

    def tool(self, name: str) -> str | None:
        return shutil.which(name)

    def root_audit(self) -> tuple[bool, list[str]]:
        missing = [rel for rel in REQUIRED_ROOT if not (self.root / rel).exists()]
        errors: list[str] = []
        if missing:
            errors.extend(f"Missing required path: {rel}" for rel in missing)
        try:
            config = json.loads((self.root / "forge.project.json").read_text(encoding="utf-8"))
            if config.get("id") != PROJECT_ID:
                errors.append("forge.project.json project id is not 'ember'")
            actual_remote = config.get("repository", {}).get("remote")
            if actual_remote != EXPECTED_REMOTE:
                errors.append(f"Configured authority mismatch: {actual_remote!r}")
        except Exception as exc:
            errors.append(f"forge.project.json invalid: {exc}")
        return not errors, errors

    def dependency_status(self) -> dict[str, str]:
        result = {
            "Python": platform.python_version(),
            "Git": "Missing",
            "Cargo": "Missing",
            "Rustc": "Missing",
        }
        for label, command in (("Git", "git"), ("Cargo", "cargo"), ("Rustc", "rustc")):
            path = self.tool(command)
            if not path:
                continue
            version = self.run([command, "--version"], live=False).output.strip()
            result[label] = version or path
        return result

    def git_info(self) -> dict[str, str]:
        info = {"state": "Not a repository", "branch": "-", "remote": "-", "head": "-", "dirty": "-"}
        if not (self.root / ".git").exists():
            return info
        info["state"] = "Repository"
        branch = self.run(["git", "branch", "--show-current"], live=False).output.strip()
        head = self.run(["git", "rev-parse", "--short", "HEAD"], live=False).output.strip()
        remote_res = self.run(["git", "remote", "get-url", "origin"], live=False)
        status_res = self.run(["git", "status", "--porcelain=v1"], live=False)
        info["branch"] = branch or "detached"
        info["head"] = head or "unborn"
        info["remote"] = remote_res.output.strip() if remote_res.returncode == 0 else "Missing"
        info["dirty"] = "Modified" if status_res.output.strip() else "Clean"
        return info

    def iter_source_files(self) -> Iterable[Path]:
        for path in sorted(self.root.rglob("*")):
            if not path.is_file():
                continue
            rel = path.relative_to(self.root)
            if any(part in EXCLUDED_ROOTS for part in rel.parts):
                continue
            yield path

    def source_fingerprint(self) -> tuple[str, int, int]:
        digest = hashlib.sha256()
        count = 0
        total = 0
        for path in self.iter_source_files():
            rel = path.relative_to(self.root).as_posix().encode("utf-8")
            data = path.read_bytes()
            digest.update(len(rel).to_bytes(4, "big"))
            digest.update(rel)
            digest.update(len(data).to_bytes(8, "big"))
            digest.update(data)
            count += 1
            total += len(data)
        return digest.hexdigest(), count, total

    def latest_green_record(self) -> dict | None:
        candidates = sorted(self.gates.glob("*_PASS.json"), reverse=True)
        for path in candidates:
            try:
                return json.loads(path.read_text(encoding="utf-8"))
            except Exception:
                continue
        return None

    def green_state(self) -> str:
        record = self.latest_green_record()
        if not record:
            return "None"
        current, _, _ = self.source_fingerprint()
        if current == record.get("source_fingerprint"):
            return f"GREEN {record.get('gate_id', '?')}"
        return "STALE (source changed)"

    def create_debug_bundle(self, reason: str, *, open_folder: bool = True) -> Path:
        bundle = self.debug_bundles / f"Ember_DebugBundle_{stamp()}_FAIL.zip"
        info = {
            "project": PROJECT_NAME,
            "project_id": PROJECT_ID,
            "created_at": iso_now(),
            "reason": reason,
            "platform": platform.platform(),
            "python": platform.python_version(),
            "root": str(self.root),
            "git": self.git_info(),
            "dependencies": self.dependency_status(),
            "green_state": self.green_state(),
        }
        with zipfile.ZipFile(bundle, "w", zipfile.ZIP_DEFLATED) as zf:
            zf.writestr("debug-info.json", json.dumps(info, indent=2) + "\n")
            for rel in ("Cargo.toml", "forge.project.json", "README.md", ".github/workflows/ember-ci.yml"):
                path = self.root / rel
                if path.exists():
                    zf.write(path, f"project/{rel}")
            for path in sorted(self.logs.glob("*.log"), reverse=True)[:5]:
                zf.write(path, f"logs/{path.name}")
            for path in sorted(self.gates.glob("*.json"), reverse=True)[:5]:
                zf.write(path, f"quality-gates/{path.name}")
            if (self.root / "Cargo.lock").exists():
                zf.write(self.root / "Cargo.lock", "project/Cargo.lock")
        self.print_status("WARN", f"Debug bundle: {bundle}")
        if open_folder:
            self.open_folder(bundle.parent)
        return bundle

    def open_folder(self, path: Path) -> None:
        if self.noninteractive:
            return
        try:
            if os.name == "nt":
                os.startfile(str(path))  # type: ignore[attr-defined]
            elif sys.platform == "darwin":
                subprocess.Popen(["open", str(path)])
            else:
                subprocess.Popen(["xdg-open", str(path)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception as exc:
            self.log(f"Could not open folder {path}: {exc}", "WARN")

    def full_gate(self) -> bool:
        print("\n" + "=" * 72)
        print(" EMBER FULL QUALITY GATE")
        print("=" * 72)
        started = time.monotonic()
        gate_id = f"QG-{stamp()}"
        record: dict = {
            "schema_version": 1,
            "project_id": PROJECT_ID,
            "gate_id": gate_id,
            "started_at": iso_now(),
            "steps": [],
            "status": "RUNNING",
        }
        ok, errors = self.root_audit()
        record["steps"].append({"name": "Root audit", "status": "PASS" if ok else "FAIL", "errors": errors})
        if not ok:
            for err in errors:
                self.print_status("FAIL", err)
            return self._finish_gate(record, False, started, "Root audit failed")
        self.print_status("PASS", "Root audit")

        deps = self.dependency_status()
        missing = [name for name in ("Git", "Cargo", "Rustc") if deps[name] == "Missing"]
        record["dependencies"] = deps
        if missing:
            message = "Missing required tools: " + ", ".join(missing)
            self.print_status("FAIL", message)
            record["steps"].append({"name": "Dependencies", "status": "FAIL", "missing": missing})
            return self._finish_gate(record, False, started, message)
        record["steps"].append({"name": "Dependencies", "status": "PASS"})
        self.print_status("PASS", "Git/Cargo/Rust toolchain available")

        steps = [
            ("Cargo fmt apply", ["cargo", "fmt", "--all"]),
            ("Cargo fmt check", ["cargo", "fmt", "--all", "--", "--check"]),
            ("Cargo check", ["cargo", "check", "--workspace", "--all-targets"]),
            ("Cargo test", ["cargo", "test", "--workspace"]),
            ("Clippy strict", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
            ("Architecture validation", ["cargo", "run", "-p", "ember_validation", "--bin", "ember_validate", "--", "architecture", "."]),
            ("Certification vertical slice", ["cargo", "test", "-p", "ember_tools", "--test", "certification"]),
            ("Build editor", ["cargo", "build", "-p", "ember_editor"]),
            ("Build runtime", ["cargo", "build", "-p", "ember_runtime_host"]),
        ]
        for name, command in steps:
            print(f"\n--- {name} {'-' * max(1, 64-len(name))}")
            result = self.run(command, live=True)
            record["steps"].append(
                {
                    "name": name,
                    "status": "PASS" if result.returncode == 0 else "FAIL",
                    "command": command,
                    "returncode": result.returncode,
                    "elapsed_seconds": round(result.elapsed, 3),
                }
            )
            if result.returncode != 0:
                self.print_status("FAIL", f"{name} failed ({result.returncode})")
                return self._finish_gate(record, False, started, f"{name} failed")
            self.print_status("PASS", f"{name} ({result.elapsed:.2f}s)")

        fingerprint, count, total = self.source_fingerprint()
        record["source_fingerprint"] = fingerprint
        record["source_file_count"] = count
        record["source_bytes"] = total
        return self._finish_gate(record, True, started, "All gate stages passed")

    def _finish_gate(self, record: dict, passed: bool, started: float, reason: str) -> bool:
        record["status"] = "PASS" if passed else "FAIL"
        record["reason"] = reason
        record["finished_at"] = iso_now()
        record["elapsed_seconds"] = round(time.monotonic() - started, 3)
        if "source_fingerprint" not in record:
            try:
                fp, count, total = self.source_fingerprint()
                record["source_fingerprint"] = fp
                record["source_file_count"] = count
                record["source_bytes"] = total
            except Exception:
                pass
        suffix = "PASS" if passed else "FAIL"
        path = self.gates / f"{record['gate_id']}_{suffix}.json"
        path.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        if passed:
            self.print_status("PASS", f"FULL QUALITY GATE {record['gate_id']} in {record['elapsed_seconds']:.2f}s")
            print(f"Gate evidence: {path}")
        else:
            self.print_status("FAIL", f"FULL QUALITY GATE {record['gate_id']}: {reason}")
            self.create_debug_bundle(reason)
        return passed

    def git_repair(self) -> bool:
        """Normalize this working folder onto the authoritative GitHub history.

        A freshly extracted Ember source tree may not have .git metadata yet. In
        that case, initialize Git, fetch origin/main, preserve the previous remote
        main on an archive branch, then anchor the local main HEAD to origin/main
        with a mixed reset. The working tree is intentionally left untouched so
        the Ember source becomes one ordinary forward commit on top of the legacy
        repository history. No force push is required.
        """
        if not self.tool("git"):
            self.print_status("FAIL", "Git is not installed.")
            return False

        created_repo = False
        if not (self.root / ".git").exists():
            res = self.run(["git", "init", "-b", DEFAULT_BRANCH])
            if res.returncode != 0:
                return False
            created_repo = True
            self.print_status("PASS", f"Initialized Git repository on {DEFAULT_BRANCH}")

        remote = self.run(["git", "remote", "get-url", "origin"], live=False)
        if remote.returncode != 0:
            if self.run(["git", "remote", "add", "origin", EXPECTED_REMOTE]).returncode != 0:
                return False
            self.print_status("PASS", f"Added origin {EXPECTED_REMOTE}")
        elif remote.output.strip() != EXPECTED_REMOTE:
            old = remote.output.strip()
            if self.run(["git", "remote", "set-url", "origin", EXPECTED_REMOTE]).returncode != 0:
                return False
            self.print_status("PASS", f"Repaired origin: {old} -> {EXPECTED_REMOTE}")
        else:
            self.print_status("PASS", "GitHub authority already correct")

        fetch = self.run(["git", "fetch", "origin", DEFAULT_BRANCH])
        if fetch.returncode != 0:
            self.print_status("FAIL", f"Could not fetch origin/{DEFAULT_BRANCH}")
            return False

        remote_head = self.run(
            ["git", "rev-parse", "--verify", f"origin/{DEFAULT_BRANCH}"],
            live=False,
        )
        if remote_head.returncode != 0:
            # A new/empty remote is valid; the first GREEN commit can create main.
            self.print_status("PASS", "Remote main does not yet exist; local GREEN commit will create it")
            return True

        local_head = self.run(["git", "rev-parse", "--verify", "HEAD"], live=False)
        if local_head.returncode != 0:
            # Fresh extracted source: preserve the legacy remote tip, then make it
            # the parent of the eventual Ember source commit without touching the
            # working tree.
            archive = f"archive/pre-ember-cutover-{stamp()}"
            if self.run(["git", "branch", archive, f"origin/{DEFAULT_BRANCH}"]).returncode != 0:
                self.print_status("FAIL", "Could not create the pre-cutover archive branch")
                return False
            archive_push = self.run(["git", "push", "origin", archive])
            if archive_push.returncode != 0:
                self.print_status("FAIL", "Could not preserve the pre-cutover GitHub branch; refusing cutover")
                return False
            self.print_status("PASS", f"Preserved previous GitHub main as {archive}")

            reset = self.run(["git", "reset", "--mixed", f"origin/{DEFAULT_BRANCH}"])
            if reset.returncode != 0:
                self.print_status("FAIL", "Could not anchor local main to origin/main")
                return False
            self.print_status(
                "PASS",
                "Anchored extracted Ember source to existing GitHub history; working tree preserved",
            )
            return True

        # Existing repository: only accept histories where origin/main is already
        # an ancestor of local HEAD (equal or local-ahead). Never silently merge,
        # reset, or force a diverged project.
        ancestor = self.run(
            ["git", "merge-base", "--is-ancestor", f"origin/{DEFAULT_BRANCH}", "HEAD"],
            live=False,
        )
        if ancestor.returncode == 0:
            return True

        local_ancestor = self.run(
            ["git", "merge-base", "--is-ancestor", "HEAD", f"origin/{DEFAULT_BRANCH}"],
            live=False,
        )
        if local_ancestor.returncode == 0:
            self.print_status(
                "FAIL",
                "GitHub main is ahead of this checkout. Reconcile/pull before committing GREEN source.",
            )
            return False

        # GREEN lineage recovery for the initial Ember GitHub cutover.
        #
        # The bootstrap/recovery passes may have produced more than one local
        # standalone Ember commit before the authoritative legacy GitHub history
        # was attached. Commit count is therefore not a safe identity test.
        #
        # Recovery is allowed only for the canonical Ember project + canonical
        # remote. commit_push_green() has already verified that the working source
        # exactly matches the latest FULL GREEN fingerprint before entering here.
        # Preserve BOTH unrelated histories remotely first, then move local main
        # with --mixed so no working-tree Ember files are replaced or deleted.
        project_manifest = self.root / "forge.project.json"
        is_ember_root = False
        try:
            project_data = json.loads(project_manifest.read_text(encoding="utf-8"))
            project_identity = str(
                project_data.get("project_id")
                or project_data.get("id")
                or project_data.get("name")
                or ""
            ).strip().lower()
            is_ember_root = project_identity == "ember"
        except Exception:
            is_ember_root = False

        remote_url = self.run(
            ["git", "remote", "get-url", "origin"], live=False
        )
        canonical_remote = (
            remote_url.returncode == 0
            and normalize_remote(remote_url.output.strip())
            == normalize_remote(DEFAULT_REMOTE)
        )

        if (
            is_ember_root
            and canonical_remote
            and (self.root / "PROJECT_CONTROL_CENTER.cmd").is_file()
            and (self.root / "Cargo.toml").is_file()
        ):
            token = stamp()
            remote_archive = f"archive/pre-ember-cutover-{token}"
            local_archive = f"archive/local-green-ember-{token}"

            if self.run(
                ["git", "branch", remote_archive, f"origin/{DEFAULT_BRANCH}"]
            ).returncode != 0:
                self.print_status("FAIL", "Could not create remote-history recovery branch")
                return False
            if self.run(["git", "push", "origin", remote_archive]).returncode != 0:
                self.print_status(
                    "FAIL",
                    "Could not preserve existing GitHub main; refusing GREEN-lineage recovery",
                )
                return False
            self.print_status("PASS", f"Preserved previous GitHub main as {remote_archive}")

            if self.run(["git", "branch", local_archive, "HEAD"]).returncode != 0:
                self.print_status("FAIL", "Could not create local GREEN-lineage recovery branch")
                return False
            if self.run(["git", "push", "origin", local_archive]).returncode != 0:
                self.print_status(
                    "FAIL",
                    "Could not preserve local GREEN Ember lineage; refusing history repair",
                )
                return False
            self.print_status("PASS", f"Preserved local GREEN Ember lineage as {local_archive}")

            reset = self.run(["git", "reset", "--mixed", f"origin/{DEFAULT_BRANCH}"])
            if reset.returncode != 0:
                self.print_status("FAIL", "Could not anchor GREEN Ember source to GitHub main")
                return False

            self.print_status(
                "PASS",
                "Recovered FULL GREEN Ember source onto authoritative GitHub history; "
                "working tree preserved for a normal forward commit",
            )
            return True

        self.print_status(
            "FAIL",
            "Local and GitHub histories diverged or are unrelated. Refusing repair because "
            "the canonical Ember project/remote identity could not be proven.",
        )
        return False

    def archive_head(self) -> bool:
        if not (self.root / ".git").exists():
            self.print_status("FAIL", "Not a Git repository")
            return False
        head = self.run(["git", "rev-parse", "HEAD"], live=False)
        if head.returncode != 0:
            self.print_status("FAIL", "No commit exists to archive")
            return False
        branch = f"archive/ember-{stamp()}"
        if self.run(["git", "branch", branch, head.output.strip()]).returncode != 0:
            return False
        self.print_status("PASS", f"Created local archive branch {branch}")
        if self.run(["git", "push", "origin", branch]).returncode == 0:
            self.print_status("PASS", f"Pushed {branch}")
            return True
        self.print_status("WARN", "Archive branch remains local because push failed")
        return False

    def pull_ff_only(self) -> bool:
        if not self.git_repair():
            return False
        info = self.git_info()
        if info["dirty"] == "Modified":
            self.print_status("FAIL", "Working tree is modified; refusing pull")
            return False
        return self.run(["git", "pull", "--ff-only", "origin", DEFAULT_BRANCH]).returncode == 0

    def commit_push_green(self, message: str | None = None) -> bool:
        # Verify GREEN before any repository-history normalization. Git metadata is
        # excluded from the source fingerprint, so a safe cutover can happen after
        # this check without invalidating the gate evidence.
        record = self.latest_green_record()
        if not record:
            self.print_status("FAIL", "No passing FULL quality gate exists. Run option 1 first.")
            return False
        current_fp, _, _ = self.source_fingerprint()
        if current_fp != record.get("source_fingerprint"):
            self.print_status("FAIL", "Source no longer matches the last FULL GREEN gate. Run option 1 again.")
            return False
        if not self.git_repair():
            return False
        info = self.git_info()
        if info["branch"] not in (DEFAULT_BRANCH, "-"):
            self.print_status("FAIL", f"Expected branch {DEFAULT_BRANCH}; current branch is {info['branch']}")
            return False
        if self.run(["git", "add", "-A"]).returncode != 0:
            return False
        staged = self.run(["git", "diff", "--cached", "--quiet"], live=False)
        if staged.returncode == 1:
            commit_message = message or f"Ember GREEN checkpoint - {now_local().strftime('%Y-%m-%d %H:%M')}"
            if self.run(["git", "commit", "-m", commit_message]).returncode != 0:
                return False
            self.print_status("PASS", "Committed current FULL GREEN source")
        elif staged.returncode == 0:
            self.print_status("PASS", "No source changes require a new commit")
        else:
            self.print_status("FAIL", "Could not determine staged Git state")
            return False
        push = self.run(["git", "push", "-u", "origin", DEFAULT_BRANCH])
        if push.returncode != 0:
            self.print_status("FAIL", "Push failed")
            self.create_debug_bundle("GitHub push failed")
            return False
        self.print_status("PASS", f"FULL GREEN source pushed to {EXPECTED_REMOTE}")
        return True

    def build_target(self, package: str) -> bool:
        if not self.tool("cargo"):
            self.print_status("FAIL", "Cargo is not installed")
            return False
        return self.run(["cargo", "build", "-p", package]).returncode == 0

    def run_target(self, package: str) -> bool:
        if not self.tool("cargo"):
            self.print_status("FAIL", "Cargo is not installed")
            return False
        return self.run(["cargo", "run", "-p", package]).returncode == 0

    def create_source_snapshot(self) -> Path:
        output = self.snapshots / f"Ember_SourceSnapshot_{stamp()}.zip"
        with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as zf:
            for path in self.iter_source_files():
                zf.write(path, path.relative_to(self.root).as_posix())
        self.print_status("PASS", f"Source snapshot created: {output} ({human_size(output.stat().st_size)})")
        return output

    @staticmethod
    def _safe_rel(value: str) -> Path:
        pure = PurePosixPath(value.replace("\\", "/"))
        if pure.is_absolute() or any(part in ("", ".", "..") for part in pure.parts):
            raise ValueError(f"Unsafe path: {value}")
        if pure.parts and pure.parts[0].lower() in {".git", "artifacts", "target"}:
            raise ValueError(f"Patch may not modify protected path: {value}")
        if ":" in pure.parts[0]:
            raise ValueError(f"Unsafe drive-qualified path: {value}")
        return Path(*pure.parts)

    def inspect_patch(self, zip_path: Path) -> tuple[dict, str] | None:
        try:
            with zipfile.ZipFile(zip_path) as zf:
                names = set(zf.namelist())
                manifest_name = next((name for name in PATCH_MANIFEST_NAMES if name in names), None)
                if not manifest_name:
                    return None
                manifest = json.loads(zf.read(manifest_name).decode("utf-8"))
                if manifest.get("schema_version") != 1:
                    raise ValueError("Unsupported patch schema_version")
                if manifest.get("project_id") != PROJECT_ID:
                    raise ValueError(f"Patch targets {manifest.get('project_id')!r}, not 'ember'")
                patch_id = str(manifest.get("patch_id") or zip_path.stem)
                files = manifest.get("files")
                if not isinstance(files, list) or not files:
                    raise ValueError("Patch manifest requires a non-empty files array")
                seen: set[str] = set()
                for entry in files:
                    if not isinstance(entry, dict):
                        raise ValueError("Each files entry must be an object")
                    rel = self._safe_rel(str(entry.get("path", ""))).as_posix()
                    if rel in seen:
                        raise ValueError(f"Duplicate patch path: {rel}")
                    seen.add(rel)
                    expected = str(entry.get("sha256", "")).lower()
                    if len(expected) != 64 or any(c not in "0123456789abcdef" for c in expected):
                        raise ValueError(f"Missing/invalid sha256 for {rel}")
                    member = f"payload/{rel}"
                    if member not in names:
                        raise ValueError(f"Missing payload member: {member}")
                    actual = hashlib.sha256(zf.read(member)).hexdigest()
                    if actual != expected:
                        raise ValueError(f"Checksum mismatch for {rel}")
                removes = manifest.get("remove", [])
                if not isinstance(removes, list):
                    raise ValueError("remove must be an array")
                for value in removes:
                    rel = self._safe_rel(str(value)).as_posix()
                    if rel in seen:
                        raise ValueError(f"Path cannot be both written and removed: {rel}")
                return manifest, patch_id
        except Exception as exc:
            raise ValueError(f"Invalid patch {zip_path.name}: {exc}") from exc

    def scan_root_patches(self) -> list[Path]:
        patches: list[Path] = []
        for path in sorted(self.root.glob("*.zip")):
            try:
                inspected = self.inspect_patch(path)
                if inspected:
                    patches.append(path)
            except Exception as exc:
                self.log(str(exc), "WARN")
                patches.append(path)
        return patches

    def apply_patch(self, zip_path: Path) -> bool:
        try:
            inspected = self.inspect_patch(zip_path)
            if not inspected:
                return False
            manifest, patch_id = inspected
        except Exception as exc:
            self.print_status("FAIL", str(exc))
            self._archive_failed_patch(zip_path, str(exc))
            self.create_debug_bundle(f"Patch validation failed: {zip_path.name}")
            return False
        tx_id = f"{stamp()}-{patch_id}"
        backup_root = self.recovery / tx_id
        backup_files = backup_root / "files"
        backup_files.mkdir(parents=True, exist_ok=True)
        state: dict = {"patch": zip_path.name, "patch_id": patch_id, "created_at": iso_now(), "previous": {}, "new_files": []}
        self.log(f"PATCH APPLY START patch={zip_path.name} id={patch_id}")
        try:
            writes: list[tuple[Path, bytes, str]] = []
            with zipfile.ZipFile(zip_path) as zf:
                for entry in manifest["files"]:
                    rel = self._safe_rel(str(entry["path"]))
                    data = zf.read(f"payload/{rel.as_posix()}")
                    writes.append((rel, data, str(entry["sha256"]).lower()))
            removes = [self._safe_rel(str(v)) for v in manifest.get("remove", [])]
            touched = [rel for rel, _, _ in writes] + removes
            for rel in touched:
                target = self.root / rel
                key = rel.as_posix()
                if target.exists() and target.is_file():
                    backup = backup_files / rel
                    backup.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(target, backup)
                    state["previous"][key] = True
                elif not target.exists():
                    state["previous"][key] = False
                else:
                    raise RuntimeError(f"Patch target is not a regular file: {rel}")
            (backup_root / "transaction.json").write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")
            for rel, data, _ in writes:
                target = self.root / rel
                target.parent.mkdir(parents=True, exist_ok=True)
                temp = target.with_name(target.name + ".ember-patch.tmp")
                temp.write_bytes(data)
                os.replace(temp, target)
            for rel in removes:
                target = self.root / rel
                if target.exists():
                    target.unlink()
            for rel, _, expected in writes:
                target = self.root / rel
                if not target.exists():
                    raise RuntimeError(f"Post-apply file missing: {rel}")
                actual = hashlib.sha256(target.read_bytes()).hexdigest()
                if actual != expected:
                    raise RuntimeError(f"Post-apply checksum mismatch: {rel}")
            for rel in removes:
                if (self.root / rel).exists():
                    raise RuntimeError(f"Post-apply removal failed: {rel}")
            archived = self.update_consumed / f"{zip_path.stem}_{stamp()}{zip_path.suffix}"
            shutil.move(str(zip_path), archived)
            self.print_status("PASS", f"Applied patch {patch_id}; archived to {archived}")
            self.log(f"PATCH APPLY PASS patch={patch_id}")
            return True
        except Exception as exc:
            self.log(f"PATCH APPLY FAIL patch={patch_id} error={exc}", "ERROR")
            self._rollback_patch(backup_root)
            self._archive_failed_patch(zip_path, str(exc))
            self.print_status("FAIL", f"Patch {patch_id} failed and was rolled back: {exc}")
            self.create_debug_bundle(f"Patch apply failed: {patch_id}")
            return False

    def _rollback_patch(self, backup_root: Path) -> None:
        tx = backup_root / "transaction.json"
        if not tx.exists():
            return
        state = json.loads(tx.read_text(encoding="utf-8"))
        for rel_text, existed in state.get("previous", {}).items():
            rel = self._safe_rel(rel_text)
            target = self.root / rel
            backup = backup_root / "files" / rel
            if existed:
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(backup, target)
            elif target.exists() and target.is_file():
                target.unlink()
        self.log(f"ROLLBACK PASS transaction={backup_root.name}")

    def _archive_failed_patch(self, zip_path: Path, reason: str) -> None:
        if not zip_path.exists():
            return
        target = self.update_failed / f"{zip_path.stem}_{stamp()}{zip_path.suffix}"
        shutil.move(str(zip_path), target)
        target.with_suffix(target.suffix + ".error.txt").write_text(reason + "\n", encoding="utf-8")

    def auto_patch_intake(self) -> bool:
        patches = self.scan_root_patches()
        if not patches:
            self.print_status("PASS", "No pending Ember root-drop patches")
            return True
        print(f"\nDetected {len(patches)} root-drop patch package(s).")
        all_ok = True
        for path in patches:
            if not self.apply_patch(path):
                all_ok = False
        return all_ok

    def install_dependencies(self) -> bool:
        status = self.dependency_status()
        missing = [name for name in ("Git", "Cargo", "Rustc") if status[name] == "Missing"]
        if not missing:
            self.print_status("PASS", "Required development dependencies are installed")
            return True
        if os.name != "nt" or not self.tool("winget"):
            self.print_status("FAIL", "Automatic bootstrap requires Windows Package Manager (winget)")
            print("Install Git and Rustup, then reopen the Project Control Center.")
            return False
        commands: list[list[str]] = []
        if status["Git"] == "Missing":
            commands.append(["winget", "install", "--id", "Git.Git", "-e", "--accept-package-agreements", "--accept-source-agreements"])
        if status["Cargo"] == "Missing" or status["Rustc"] == "Missing":
            commands.append(["winget", "install", "--id", "Rustlang.Rustup", "-e", "--accept-package-agreements", "--accept-source-agreements"])
        for cmd in commands:
            if self.run(cmd).returncode != 0:
                self.print_status("FAIL", f"Dependency install failed: {cmd[-2] if len(cmd) > 2 else cmd[0]}")
                return False
        self.print_status("PASS", "Dependency installers completed. Reopen this terminal so PATH changes take effect.")
        return True

    def header(self) -> None:
        info = self.git_info()
        deps = self.dependency_status()
        pending = len(self.scan_root_patches())
        green = self.green_state()
        print("\n" + "=" * 72)
        print(" EMBER PROJECT CONTROL CENTER")
        print("=" * 72)
        print(f" Repository : {self.root}")
        print(f" Git        : {info['branch']} / {info['dirty']} / {info['head']}")
        print(f" GitHub     : {info['remote']}")
        print(f" Gate       : {green}")
        print(f" Updates    : {pending} pending")
        print(f" Cargo      : {'Ready' if deps['Cargo'] != 'Missing' else 'Missing'}")
        print(f" Active log : {self.log_path}")
        print("-" * 72)
        print("  1. FULL QUALITY GATE")
        print("  2. COMMIT + PUSH FULL GREEN")
        print("")
        print("  3. Build / Run")
        print("  4. Source Control / GitHub")
        print("  5. Updates / Patch Intake")
        print("  6. Project / Dependencies")
        print("  7. Diagnostics / Artifacts")
        print("  8. Advanced")
        print("")
        print("  0. Exit")

    def interactive(self) -> int:
        # Root-drop intake occurs before normal project operations.
        self.auto_patch_intake()
        while True:
            self.header()
            choice = input("Select an option: ").strip().lower()
            if choice == "1":
                self.full_gate()
            elif choice == "2":
                default = f"Ember GREEN checkpoint - {now_local().strftime('%Y-%m-%d %H:%M')}"
                message = input(f"Commit message [{default}]: ").strip() or default
                self.commit_push_green(message)
            elif choice == "3":
                self.menu_build_run()
            elif choice == "4":
                self.menu_git()
            elif choice == "5":
                self.menu_updates()
            elif choice == "6":
                self.menu_project()
            elif choice == "7":
                self.menu_diagnostics()
            elif choice == "8":
                self.menu_advanced()
            elif choice in ("0", "q", "quit", "exit"):
                return 0
            else:
                print("Unknown selection.")

    def menu_build_run(self) -> None:
        while True:
            print("\nBUILD / RUN")
            print("  1. Build editor")
            print("  2. Build runtime host")
            print("  3. Run editor")
            print("  4. Run runtime host")
            print("  5. Cargo check workspace")
            print("  0. Back")
            c = input("Select: ").strip()
            if c == "1": self.build_target("ember_editor")
            elif c == "2": self.build_target("ember_runtime_host")
            elif c == "3": self.run_target("ember_editor")
            elif c == "4": self.run_target("ember_runtime_host")
            elif c == "5": self.run(["cargo", "check", "--workspace", "--all-targets"])
            elif c == "0": return

    def menu_git(self) -> None:
        while True:
            print("\nSOURCE CONTROL / GITHUB")
            print("  1. Status / branch / remote")
            print("  2. Initialize / repair GitHub authority")
            print("  3. Pull origin/main (fast-forward only)")
            print("  4. Review working changes")
            print("  5. Commit + push current FULL GREEN")
            print("  6. Push main")
            print("  7. Preserve current HEAD as archive branch")
            print("  0. Back")
            c = input("Select: ").strip()
            if c == "1": print(json.dumps(self.git_info(), indent=2))
            elif c == "2": self.git_repair()
            elif c == "3": self.pull_ff_only()
            elif c == "4": self.run(["git", "status"]); self.run(["git", "diff", "--stat"])
            elif c == "5": self.commit_push_green()
            elif c == "6": self.run(["git", "push", "origin", DEFAULT_BRANCH])
            elif c == "7": self.archive_head()
            elif c == "0": return

    def menu_updates(self) -> None:
        while True:
            print("\nUPDATES / PATCH INTAKE")
            print("  1. Scan + apply root-drop patches")
            print("  2. Open consumed patch archive")
            print("  3. Open failed patch archive")
            print("  4. Open recovery transactions")
            print("  0. Back")
            c = input("Select: ").strip()
            if c == "1": self.auto_patch_intake()
            elif c == "2": self.open_folder(self.update_consumed)
            elif c == "3": self.open_folder(self.update_failed)
            elif c == "4": self.open_folder(self.recovery)
            elif c == "0": return

    def menu_project(self) -> None:
        while True:
            print("\nPROJECT / DEPENDENCIES")
            print("  1. Root audit")
            print("  2. Dependency status")
            print("  3. Install missing Git/Rust dependencies with winget")
            print("  4. Show project manifest")
            print("  0. Back")
            c = input("Select: ").strip()
            if c == "1":
                ok, errors = self.root_audit(); print("PASS" if ok else "FAIL"); [print(" -", e) for e in errors]
            elif c == "2": print(json.dumps(self.dependency_status(), indent=2))
            elif c == "3": self.install_dependencies()
            elif c == "4": print((self.root / "forge.project.json").read_text(encoding="utf-8"))
            elif c == "0": return

    def menu_diagnostics(self) -> None:
        while True:
            print("\nDIAGNOSTICS / ARTIFACTS")
            print("  1. Create debug bundle")
            print("  2. Create source snapshot")
            print("  3. Open artifacts")
            print("  4. Open latest session log")
            print("  5. Show current GREEN evidence")
            print("  0. Back")
            c = input("Select: ").strip()
            if c == "1": self.create_debug_bundle("Manual diagnostic bundle")
            elif c == "2": self.create_source_snapshot()
            elif c == "3": self.open_folder(self.artifacts)
            elif c == "4": self.open_folder(self.log_path.parent)
            elif c == "5": print(json.dumps(self.latest_green_record(), indent=2) if self.latest_green_record() else "No GREEN gate yet.")
            elif c == "0": return

    def menu_advanced(self) -> None:
        while True:
            print("\nADVANCED")
            print("  1. Source fingerprint")
            print("  2. Archive current Git HEAD")
            print("  3. Cargo clean")
            print("  4. Open repository root")
            print("  0. Back")
            c = input("Select: ").strip()
            if c == "1":
                fp, count, total = self.source_fingerprint(); print(f"{fp}\n{count} files / {human_size(total)}")
            elif c == "2": self.archive_head()
            elif c == "3": self.run(["cargo", "clean"])
            elif c == "4": self.open_folder(self.root)
            elif c == "0": return


def find_root(script: Path) -> Path:
    return script.resolve().parents[2]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Ember project-local Project Control Center")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--status", action="store_true")
    group.add_argument("--full-gate", action="store_true")
    group.add_argument("--commit-push-green", action="store_true")
    group.add_argument("--git-repair", action="store_true")
    group.add_argument("--patch-intake", action="store_true")
    group.add_argument("--debug-bundle", action="store_true")
    group.add_argument("--source-snapshot", action="store_true")
    group.add_argument("--build-editor", action="store_true")
    group.add_argument("--build-runtime", action="store_true")
    group.add_argument("--run-editor", action="store_true")
    group.add_argument("--run-runtime", action="store_true")
    group.add_argument("--install-dependencies", action="store_true")
    group.add_argument("--architecture-validate", action="store_true")
    group.add_argument("--certification", action="store_true")
    parser.add_argument("--message", help="Commit message for --commit-push-green")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = find_root(Path(__file__))
    noninteractive = any(
        getattr(args, name)
        for name in (
            "status", "full_gate", "commit_push_green", "git_repair", "patch_intake",
            "debug_bundle", "source_snapshot", "build_editor", "build_runtime", "run_editor",
            "run_runtime", "install_dependencies", "architecture_validate", "certification"
        )
    )
    pcc = PCC(root, noninteractive=noninteractive)
    try:
        if args.status:
            print(json.dumps({
                "project": PROJECT_NAME,
                "root": str(root),
                "git": pcc.git_info(),
                "dependencies": pcc.dependency_status(),
                "green_state": pcc.green_state(),
                "pending_patches": len(pcc.scan_root_patches()),
            }, indent=2))
            return 0
        if args.full_gate: return 0 if pcc.full_gate() else 1
        if args.commit_push_green: return 0 if pcc.commit_push_green(args.message) else 1
        if args.git_repair: return 0 if pcc.git_repair() else 1
        if args.patch_intake: return 0 if pcc.auto_patch_intake() else 1
        if args.debug_bundle: pcc.create_debug_bundle("Manual CLI diagnostic bundle", open_folder=False); return 0
        if args.source_snapshot: pcc.create_source_snapshot(); return 0
        if args.build_editor: return 0 if pcc.build_target("ember_editor") else 1
        if args.build_runtime: return 0 if pcc.build_target("ember_runtime_host") else 1
        if args.run_editor: return 0 if pcc.run_target("ember_editor") else 1
        if args.run_runtime: return 0 if pcc.run_target("ember_runtime_host") else 1
        if args.install_dependencies: return 0 if pcc.install_dependencies() else 1
        if args.architecture_validate:
            return pcc.run(["cargo", "run", "-p", "ember_validation", "--bin", "ember_validate", "--", "architecture", "."]).returncode
        if args.certification:
            return pcc.run(["cargo", "test", "-p", "ember_tools", "--test", "certification"]).returncode
        return pcc.interactive()
    except KeyboardInterrupt:
        print("\nCancelled.")
        return 130
    except Exception as exc:
        pcc.log(traceback.format_exc(), "ERROR")
        print(f"{C.RED}[FAIL]{C.RESET} Project Control Center error: {exc}")
        try:
            pcc.create_debug_bundle(str(exc), open_folder=not noninteractive)
        except Exception:
            pass
        return 1
    finally:
        pcc.close()


if __name__ == "__main__":
    raise SystemExit(main())
