from __future__ import annotations

import importlib
import os
import sys
from pathlib import Path

from patch_engine import PatchEngine


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def restart_self() -> None:
    os.execv(sys.executable, [sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]])


def _install_runtime_bridge(legacy, root: Path) -> None:
    def make_engine(host) -> PatchEngine:
        return PatchEngine(
            root,
            status=host.print_status,
            log=host.log,
            interactive=not host.noninteractive,
        )

    def inspect_patch(host, zip_path):
        return make_engine(host).inspect_patch(Path(zip_path))

    def scan_root_patches(host):
        return make_engine(host).scan_root_patches()

    def apply_patch(host, zip_path):
        engine = make_engine(host)
        result = engine.apply_patch(Path(zip_path))
        if result.restart_required:
            restart_self()
        for lifecycle in engine.pending_lifecycle_paths():
            if not engine.run_lifecycle(host, lifecycle):
                return False
        return result.ok

    def auto_patch_intake(host):
        engine = make_engine(host)
        patches = engine.scan_root_patches()
        if not patches:
            host.print_status("PASS", "No pending Ember root-drop patches")
            return True
        print(f"\nDetected {len(patches)} root-drop patch package(s).")
        for path in patches:
            result = engine.apply_patch(path)
            if not result.ok:
                try:
                    host.create_debug_bundle(f"Patch apply failed: {result.patch_id}")
                except Exception:
                    pass
                return False
            if result.restart_required:
                restart_self()
            for lifecycle in engine.pending_lifecycle_paths():
                if not engine.run_lifecycle(host, lifecycle):
                    return False
        return True

    legacy.PCC.inspect_patch = inspect_patch
    legacy.PCC.scan_root_patches = scan_root_patches
    legacy.PCC.apply_patch = apply_patch
    legacy.PCC.auto_patch_intake = auto_patch_intake


def main() -> int:
    root = repo_root()
    engine = PatchEngine(root, interactive=not any(arg.startswith("--") for arg in sys.argv[1:]))

    # Root-drop packages are processed before pending recovery/migration plans.
    # This is intentional: if a pending plan itself is defective, a corrective
    # root-drop patch must still be able to repair the patch engine or replace
    # that plan instead of deadlocking the PCC at startup.
    root_results = engine.process_root_patches()
    for result in root_results:
        if not result.ok:
            engine.create_handoff(
                patch_id=result.patch_id,
                status="FAIL",
                reason=result.error or "root-drop patch failed",
                transaction_path=result.transaction_path,
                open_handoff=True,
            )
            return 1
        if result.restart_required:
            restart_self()

    # Recovery/migration plans live in project-local .ember state. They are
    # declarative schema-v2 manifests, never executable scripts.
    for plan_path in engine.pending_plan_paths():
        result = engine.apply_plan_file(plan_path)
        if not result.ok:
            engine.create_handoff(
                patch_id=result.patch_id,
                status="FAIL",
                reason=result.error or "pending patch plan failed",
                transaction_path=result.transaction_path,
                open_handoff=True,
            )
            return 1
        if result.restart_required:
            restart_self()

    # Import after patch intake so project operations always run the newest PCC.
    tool_dir = Path(__file__).resolve().parent
    if str(tool_dir) not in sys.path:
        sys.path.insert(0, str(tool_dir))
    legacy = importlib.import_module("ember_pcc")
    _install_runtime_bridge(legacy, root)

    lifecycle_paths = engine.pending_lifecycle_paths()
    lifecycle_ran_full_gate = False
    lifecycle_ok = True
    if lifecycle_paths:
        host = legacy.PCC(root, noninteractive=False)
        try:
            lifecycle_engine = PatchEngine(
                root,
                status=host.print_status,
                log=host.log,
                interactive=True,
            )
            for lifecycle in lifecycle_paths:
                try:
                    import json
                    record = json.loads(lifecycle.read_text(encoding="utf-8"))
                    if str((record.get("post_apply") or {}).get("run", "none")) == "full_gate":
                        lifecycle_ran_full_gate = True
                except Exception:
                    pass
                if not lifecycle_engine.run_lifecycle(host, lifecycle):
                    lifecycle_ok = False
                    break
        finally:
            host.close()

    # If the user launched an explicit full gate and the applied patch already
    # required/ran one, do not run it twice.
    if "--full-gate" in sys.argv[1:] and lifecycle_ran_full_gate:
        return 0 if lifecycle_ok else 1
    if not lifecycle_ok and any(arg.startswith("--") for arg in sys.argv[1:]):
        return 1

    return int(legacy.main())


if __name__ == "__main__":
    raise SystemExit(main())
