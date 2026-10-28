#!/usr/bin/env python3
"""Capture read-only downstream API fixture checks against baseline/current.

This runner is intentionally not executed during fixture authoring. Run only
after the current validation source is committed and read back from its remote
branch. Cargo outputs and an archived baseline source tree go outside checkout.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tarfile
from datetime import datetime, timezone


BASELINE = "3064d453091af1dec7f99f0c8afabd98b447ea34"
NATIVE_LOCK = "/tmp/vigilode-cargo-20261027.lock"
CARGO_TIMEOUT_SECONDS = 900
FIXTURES = {
    "legacy-config-literal": "legacy_config_literal.rs",
    "legacy-exhaustive-evidence-match": "legacy_exhaustive_evidence_match.rs",
}


def run(argv: list[str], *, cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, check=False)


def require_git(repo: Path, *args: str) -> str:
    result = run(["git", *args], cwd=repo)
    if result.returncode:
        raise RuntimeError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def pinned_blob(repo: Path, revision: str, relative_path: str) -> bytes:
    result = subprocess.run(["git", "show", f"{revision}:{relative_path}"], cwd=repo,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if result.returncode:
        raise RuntimeError(
            f"INPUT_DEPENDENCY: cannot read {relative_path} from {revision}: "
            f"{result.stderr.decode(errors='replace').strip()}"
        )
    return result.stdout


def archive_commit(repo: Path, revision: str, destination: Path, archive_path: Path) -> dict[str, object]:
    destination.mkdir(parents=True, exist_ok=False)
    with archive_path.open("xb") as archive_stream:
        archive = subprocess.run(["git", "archive", "--format=tar", revision], cwd=repo,
                                 stdout=archive_stream, stderr=subprocess.PIPE, check=False)
    if archive.returncode:
        raise RuntimeError(f"git archive {revision} failed: {archive.stderr.decode(errors='replace')}")
    with tarfile.open(archive_path, mode="r:") as tar:
        if hasattr(tarfile, "data_filter"):
            tar.extractall(destination, filter="data")
        else:
            tar.extractall(destination)
    required = ["Cargo.toml", "Cargo.lock", ".cargo", "crates", "fixtures", "research", "docs", "tools"]
    missing = [name for name in required if not (destination / name).exists()]
    if missing:
        return {"status": "INPUT_DEPENDENCY", "missing_paths": missing,
                "archive_sha256": sha256(archive_path), "archive_bytes": archive_path.stat().st_size}
    return {"status": "ARCHIVED", "archive_sha256": sha256(archive_path),
            "archive_bytes": archive_path.stat().st_size}


def setup_fixture(source_root: Path, fixture_root: Path, pinned_inputs: dict[str, bytes]) -> dict[str, str]:
    fixture_root.mkdir(parents=True, exist_ok=False)
    template = pinned_inputs["Cargo.toml.template"].decode("utf-8")
    manifest = template.replace("__VIGILODE_PATH__", str((source_root / "crates/vigilode").resolve()))
    (fixture_root / "Cargo.toml").write_text(manifest, encoding="utf-8")
    for source in FIXTURES.values():
        (fixture_root / source).write_bytes(pinned_inputs[source])

    lock_source = source_root / "Cargo.lock"
    lock_text = lock_source.read_text(encoding="utf-8")
    package_name = 'name = "vigilode-api-compat-consumer"'
    if package_name in lock_text:
        raise RuntimeError(f"fixture lock entry already exists in {lock_source}")
    lock_text += '''\n[[package]]
name = "vigilode-api-compat-consumer"
version = "0.0.0"
dependencies = [
 "vigilode",
]
'''
    (fixture_root / "Cargo.lock").write_text(lock_text, encoding="utf-8")
    return {"cargo_lock_sha256": sha256(lock_source), "fixture_lock_sha256": sha256(fixture_root / "Cargo.lock")}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True, help="full local VigilODE checkout")
    parser.add_argument("--source-sha", required=True, help="committed current source SHA")
    parser.add_argument("--remote-ref", required=True, help="published validation ref to read back")
    parser.add_argument("--output", type=Path, required=True, help="new output directory outside checkout")
    args = parser.parse_args()

    repo = args.repo.resolve()
    output = args.output.resolve()
    if output == repo or repo in output.parents:
        parser.error("--output must be outside the checkout")
    if output.exists():
        parser.error(f"--output already exists; choose a new path: {output}")

    actual_head = require_git(repo, "rev-parse", "HEAD")
    remote_head = require_git(repo, "rev-parse", args.remote_ref)
    if actual_head != args.source_sha or remote_head != args.source_sha:
        parser.error(f"source pin mismatch: HEAD={actual_head}, {args.remote_ref}={remote_head}, expected={args.source_sha}")
    input_paths = {
        "run_fixtures.py": Path(__file__).resolve().relative_to(repo).as_posix(),
        "Cargo.toml.template": (Path(__file__).resolve().parent / "Cargo.toml.template").relative_to(repo).as_posix(),
        **{name: (Path(__file__).resolve().parent / name).relative_to(repo).as_posix() for name in FIXTURES.values()},
    }
    pinned_inputs: dict[str, bytes] = {}
    input_hashes: dict[str, dict[str, str]] = {}
    for name, relative_path in input_paths.items():
        pinned = pinned_blob(repo, args.source_sha, relative_path)
        local = (repo / relative_path).read_bytes()
        pinned_hash = sha256_bytes(pinned)
        local_hash = sha256_bytes(local)
        if pinned_hash != local_hash:
            parser.error(
                f"SOURCE_IDENTITY_MISMATCH: working input {relative_path} sha256={local_hash} "
                f"does not match {args.source_sha} blob sha256={pinned_hash}"
            )
        if name != "run_fixtures.py":
            pinned_inputs[name] = pinned
        input_hashes[name] = {"path": relative_path, "sha256": pinned_hash}

    output.mkdir(parents=True, exist_ok=False)
    baseline_root = output / "baseline-source"
    current_root = output / "current-source"
    baseline_archive = archive_commit(repo, BASELINE, baseline_root, output / "baseline-source.tar")
    current_archive = archive_commit(repo, args.source_sha, current_root, output / "current-source.tar")
    evidence: dict[str, object] = {
        "schema": "vigilode.downstream-api-fixtures.v1",
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "baseline_sha": BASELINE,
        "current_sha": args.source_sha,
        "remote_ref": args.remote_ref,
        "remote_ref_sha": remote_head,
        "baseline_archive": baseline_archive,
        "current_archive": current_archive,
        "pinned_fixture_inputs": input_hashes,
        "runner_status": "RUNNING",
        "commands": [],
        "legacy_json_deserialize": {
            "status": "NOT_APPLICABLE",
            "reason": "The 3064d453 baseline SwitchingConfig derives Serialize only; it has no Deserialize implementation to compare.",
        },
        "interpretation": "Fixture runner completion is not a compatibility approval.",
    }
    if baseline_archive["status"] == "INPUT_DEPENDENCY" or current_archive["status"] == "INPUT_DEPENDENCY":
        evidence["runner_status"] = "INPUT_DEPENDENCY"
        evidence["compatibility_status"] = "INCONCLUSIVE_INPUT_DEPENDENCY"
        evidence["commands"] = []
        (output / "SUMMARY.json").write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps({key: evidence[key] for key in ("runner_status", "compatibility_status", "baseline_sha", "current_sha")}, sort_keys=True))
        return 2

    rust_env = os.environ.copy()
    rust_env.update({"CARGO_NET_OFFLINE": "true", "CARGO_BUILD_JOBS": "2", "RUSTUP_TOOLCHAIN": "1.94.1"})
    for lane, source_root in (("baseline", baseline_root), ("current", current_root)):
        for fixture_name in FIXTURES:
            fixture_root = output / "consumers" / lane / fixture_name
            lock_hashes = setup_fixture(source_root, fixture_root, pinned_inputs)
            target = output / "targets" / lane / fixture_name
            target.mkdir(parents=True, exist_ok=False)
            env = rust_env | {"CARGO_TARGET_DIR": str(target)}
            cargo_argv = ["cargo", "check", "--locked", "--offline", "--manifest-path",
                          str(fixture_root / "Cargo.toml"), "--bin", fixture_name]
            argv = ["flock", "-x", NATIVE_LOCK, "timeout", "--verbose", "--signal=TERM",
                    "--kill-after=10s", f"{CARGO_TIMEOUT_SECONDS}s", *cargo_argv]
            try:
                result = run(argv, cwd=fixture_root, env=env)
                exit_code: int | None = result.returncode
                stdout, stderr = result.stdout, result.stderr
                timed_out = result.returncode == 124 or "timeout: sending signal TERM" in result.stderr
                status = "TIMEOUT" if timed_out else ("COMPLETED" if result.returncode == 0 else "COMMAND_FAILED")
            except OSError as error:
                exit_code = None
                stdout = ""
                stderr = f"{type(error).__name__}: {error}\n"
                status = "ENVIRONMENT_FAILURE"
            stem = output / "commands" / lane / fixture_name
            stem.parent.mkdir(parents=True, exist_ok=True)
            (stem.with_suffix(".stdout")).write_text(stdout, encoding="utf-8")
            (stem.with_suffix(".stderr")).write_text(stderr, encoding="utf-8")
            (stem.with_suffix(".exit")).write_text(
                "UNAVAILABLE\n" if exit_code is None else f"{exit_code}\n", encoding="ascii"
            )
            evidence["commands"].append({
                "lane": lane,
                "fixture": fixture_name,
                "argv": argv,
                "cargo_argv": cargo_argv,
                "cwd": str(fixture_root),
                "status": status,
                "exit_code": exit_code,
                "timeout_seconds": CARGO_TIMEOUT_SECONDS,
                "native_lock": NATIVE_LOCK,
                "stdout": str(stem.with_suffix(".stdout").relative_to(output)),
                "stderr": str(stem.with_suffix(".stderr").relative_to(output)),
                "exit_receipt": str(stem.with_suffix(".exit").relative_to(output)),
                "targeted_diagnostic_observed": False,
                **lock_hashes,
            })

    outcomes = {(row["lane"], row["fixture"]): row["exit_code"] for row in evidence["commands"]}
    baseline_pass = all(outcomes.get(("baseline", name)) == 0 for name in FIXTURES)
    diagnostic_patterns = {
        "legacy-config-literal": re.compile(
            r"error\[E0063\].*?missing fields .*? in initializer of .*SwitchingConfig.*?legacy_config_literal\.rs", re.S
        ),
        "legacy-exhaustive-evidence-match": re.compile(
            r"error\[E0004\].*?non-exhaustive patterns.*?legacy_exhaustive_evidence_match\.rs", re.S
        ),
    }
    for row in evidence["commands"]:
        if row["lane"] == "current" and row["status"] == "COMMAND_FAILED":
            diagnostic = (output / row["stderr"]).read_text(encoding="utf-8", errors="replace")
            row["targeted_diagnostic_observed"] = bool(
                diagnostic_patterns[row["fixture"]].search(diagnostic)
            )
    current_targeted_breaks = all(
        outcomes.get(("current", name)) not in (None, 0, 124)
        and next(row for row in evidence["commands"]
                 if row["lane"] == "current" and row["fixture"] == name)["targeted_diagnostic_observed"]
        for name in FIXTURES
    )
    any_timeout = any(row["status"] == "TIMEOUT" for row in evidence["commands"])
    any_environment_failure = any(row["status"] == "ENVIRONMENT_FAILURE" for row in evidence["commands"])
    if any_timeout:
        evidence["compatibility_status"] = "INCONCLUSIVE_TIMEOUT"
    elif any_environment_failure:
        evidence["compatibility_status"] = "INCONCLUSIVE_ENVIRONMENT_FAILURE"
    elif baseline_pass and current_targeted_breaks:
        evidence["compatibility_status"] = "BREAKING_CHANGE_REPRODUCED"
    elif not baseline_pass:
        evidence["compatibility_status"] = "INCONCLUSIVE_BASELINE_FIXTURE_FAILURE"
    elif not current_targeted_breaks:
        evidence["compatibility_status"] = "INCONCLUSIVE_TARGETED_DIAGNOSTIC_NOT_OBSERVED"
    evidence["runner_status"] = "COMPLETE"
    (output / "SUMMARY.json").write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({key: evidence[key] for key in ("runner_status", "compatibility_status", "baseline_sha", "current_sha")}, sort_keys=True))
    return 0 if evidence["compatibility_status"] == "BREAKING_CHANGE_REPRODUCED" else 2


if __name__ == "__main__":
    sys.exit(main())
