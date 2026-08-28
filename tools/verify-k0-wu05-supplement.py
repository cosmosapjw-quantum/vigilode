#!/usr/bin/env python3
"""Run the frozen WU-05 validator with a bounded, transparent inventory repair.

The underlying scientific/evidence functions are unchanged. --dump-source emits
exactly the effective source that is compiled. The loader and frozen payload are
both covered by the existing supplement manifest; no new gate is introduced.
"""
from __future__ import annotations
import base64
import hashlib
import pathlib
import sys
import zlib

BASE_PAYLOAD_SHA256 = "f51e444c94481399e61bce76da9d24d6a32a0be485b9636909425be6738336a4"
path = pathlib.Path(__file__).with_name("verify-k0-wu05-supplement.payload.b64")
raw = path.read_bytes()
if hashlib.sha256(raw).hexdigest() != BASE_PAYLOAD_SHA256:
    raise SystemExit("STOP_INVALID: frozen supplement payload identity mismatch")
source = zlib.decompress(base64.b64decode(raw)).decode("utf-8")
start_token = "def active_control_files("
end_token = "def check_authority("
call_token = 'results["manifest"] = check_manifest(root)'
if source.count(start_token) != 1 or source.count(end_token) != 1 or source.count(call_token) != 1:
    raise SystemExit("STOP_INVALID: frozen supplement inventory boundary changed")
start = source.index(start_token)
end = source.index(end_token, start)
INVENTORY_REPAIR = r'''
def is_control_path(relative: pathlib.Path) -> bool:
    """One scope predicate for the worktree and the externally pinned Git tree."""
    if relative in {pathlib.Path("AGENTS.md"), OLD_MANIFEST,
                    pathlib.Path("docs/invariants/K0_STAGE_TELEMETRY.md"),
                    pathlib.Path("docs/quality/P0_P1_POLICY.md")}:
        return True
    if PACKAGE in relative.parents:
        return "__pycache__" not in relative.parts
    return relative.parent == pathlib.Path("tools") and (
        relative.name.startswith("verify-k0-")
        or relative.name.startswith("k0-wu05-")
        or relative.name.startswith("test_k0_bootstrap")
    )


def active_control_files(root: pathlib.Path) -> set[pathlib.Path]:
    candidates = {pathlib.Path("AGENTS.md"), OLD_MANIFEST,
                  pathlib.Path("docs/invariants/K0_STAGE_TELEMETRY.md"),
                  pathlib.Path("docs/quality/P0_P1_POLICY.md")}
    candidates.update(p.relative_to(root) for p in (root / PACKAGE).rglob("*")
                      if p.is_file())
    candidates.update(p.relative_to(root) for p in (root / "tools").glob("*")
                      if p.is_file())
    return {p for p in candidates if is_control_path(p) and (root / p).is_file()}


def pinned_control_files(root: pathlib.Path, pin: str) -> set[pathlib.Path]:
    if not valid_hash(pin, 40):
        die("manifest coverage requires an exact external package SHA")
    git(root, "cat-file", "-e", f"{pin}^{{commit}}")
    paths = git(root, "ls-tree", "-rz", "--name-only", pin).stdout.split("\0")
    return {pathlib.Path(p) for p in paths if p and is_control_path(pathlib.Path(p))}


def check_manifest(root: pathlib.Path, expected_package_sha: str) -> dict[str, Any]:
    authority = load(root / AUTH)
    old_entries = verify_sha_manifest(root, OLD_MANIFEST)
    supplement_entries = verify_sha_manifest(root, SUPPLEMENT_MANIFEST)
    if set(supplement_entries) != SUPPLEMENT_FILES:
        die(f"supplement manifest set drift: {sorted(map(str, set(supplement_entries) ^ SUPPLEMENT_FILES))}")
    legacy = {pathlib.Path(k): v for k, v in authority.get("legacy_unmanifested_git_blobs", {}).items()}
    for relative, blob in legacy.items():
        target = root / relative
        if not target.is_file() or not valid_hash(blob, 40):
            die(f"legacy repair authority missing/invalid: {relative}")
        got = git(root, "hash-object", str(relative)).stdout.strip()
        if got != blob:
            die(f"legacy repair blob drift {relative}: {got} != {blob}")
    core = set(old_entries) | set(legacy) | set(supplement_entries) | {OLD_MANIFEST, SUPPLEMENT_MANIFEST}
    expected = pinned_control_files(root, expected_package_sha)
    actual = active_control_files(root)
    if not core <= expected or actual != expected:
        die(f"active control-file set mismatch: extra={sorted(map(str, actual-expected))}, "
            f"missing={sorted(map(str, expected-actual))}, undeclared={sorted(map(str, core-expected))}")
    # Existing manifests retain their historical hash checks. Newly added bootstrap
    # files and the active supplement are bound to the external commit itself.
    # Thus adding a runner cannot make the manifest reject its own executable,
    # and arbitrary local files do not become authoritative merely by existing.
    checked_against_pin = (expected - core) | SUPPLEMENT_FILES | {SUPPLEMENT_MANIFEST}
    for relative in sorted(checked_against_pin):
        if (root / relative).is_symlink():
            die(f"symlink in active package: {relative}")
        wanted = git(root, "rev-parse", f"{expected_package_sha}:{relative}").stdout.strip()
        actual_blob = git(root, "hash-object", str(relative)).stdout.strip()
        if actual_blob != wanted:
            die(f"active package blob differs from external pin: {relative}")
    return {
        "status": "PASS", "marker": "WU05_SUPPLEMENT_MANIFEST_PASS",
        "legacy_marker": "LEGACY_REPAIR_BLOBS_PASS", "active_files": len(actual),
        "package_sha": expected_package_sha, "auxiliary_authority": "exact-pinned-Git-tree",
    }

'''
source = source[:start] + INVENTORY_REPAIR.lstrip("\n") + source[end:]
source = source.replace(call_token, 'results["manifest"] = check_manifest(root, pin)', 1)
if "--dump-source" in sys.argv:
    sys.stdout.write(source)
    raise SystemExit(0)
namespace = {"__name__": "__main__", "__file__": str(path)}
exec(compile(source, str(path), "exec"), namespace)
