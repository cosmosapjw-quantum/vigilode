"""Portable, lease-bound execution of compiled Rust component contracts.

This utility consumes an existing persistent P1 host lease. It never reserves,
closes, retries, or interprets a component test as a scientific admission.
"""
from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import math
import os
import platform
import re
import signal
import subprocess
import time
from pathlib import Path

from .budget import CAP_SECONDS, PHASE_SECONDS, boot_id, locked_ledger, usage
from . import hpc as hpc_resources
from .protocol import canonical, digest, file_digest, strict_load
from .runner import atomic_json

SCHEMA = "vigilode-component-contract-execution-v1"
TIMEOUT_SECONDS = 60.0
CLEANUP_SECONDS = 30.0
_SHA = re.compile(r"^[0-9a-f]{64}$")
_RESULT = re.compile(r"test result: (ok|FAILED)\.\s*(\d+) passed;\s*(\d+) failed;\s*(\d+) ignored;\s*(\d+) measured;\s*(\d+) filtered out")
_TEST = re.compile(r"^(.+): (?:test|benchmark)$")


def _source_root(manifest):
    root = Path(manifest.get("source_root", ".")).resolve()
    if not (root / ".git").exists():
        # Linked worktrees have a .git file, and normal checkouts a directory.
        raise ValueError("source_root is not a git checkout")
    return root


def _validate_manifest(manifest):
    if not isinstance(manifest, dict) or manifest.get("schema") != SCHEMA:
        raise ValueError("invalid component manifest schema")
    if not isinstance(manifest.get("exec_sha"), str) or not re.fullmatch(r"[0-9a-f]{40}", manifest["exec_sha"]):
        raise ValueError("manifest requires full EXEC SHA")
    entries = manifest.get("entries")
    if not isinstance(entries, list) or not entries:
        raise ValueError("manifest requires entries")
    seen_ids, seen_commands = set(), set()
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("id"), str) or not re.fullmatch(r"[A-Za-z0-9_-]+", entry["id"]):
            raise ValueError("invalid component id")
        if entry["id"] in seen_ids:
            raise ValueError("duplicate component id")
        seen_ids.add(entry["id"])
        executable = entry.get("executable")
        if not isinstance(executable, str) or not Path(executable).is_absolute():
            raise ValueError("component executable must be absolute")
        if not isinstance(entry.get("sha256"), str) or not _SHA.fullmatch(entry["sha256"]):
            raise ValueError("invalid executable SHA256")
        if entry.get("argv") != ["--test-threads=1"]:
            raise ValueError("component argv must be exactly --test-threads=1")
        sources = entry.get("source_paths")
        hashes = entry.get("source_hashes")
        if not isinstance(sources, list) or not sources or any(not isinstance(x, str) for x in sources) or len(set(sources)) != len(sources) or not isinstance(hashes, dict) or set(hashes) != set(sources):
            raise ValueError("source paths and hashes must match")
        for source in sources:
            path = Path(source)
            if path.is_absolute() or ".." in path.parts or not path.parts or not _SHA.fullmatch(str(hashes[source])):
                raise ValueError("invalid source binding")
            if "holdout" in source.casefold():
                raise ValueError("protected holdout source in component manifest")
        command_key = (executable, entry["sha256"], tuple(entry["argv"]))
        if command_key in seen_commands:
            raise ValueError("duplicate component executable")
        seen_commands.add(command_key)
    return entries


def _check_source(manifest):
    root = _source_root(manifest)
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, capture_output=True, text=True, check=True).stdout.strip()
    if head != manifest["exec_sha"]:
        raise RuntimeError("EXEC_SHA_MISMATCH")
    for entry in manifest["entries"]:
        for source in entry["source_paths"]:
            path = root / source
            if not path.is_file() or file_digest(path) != entry["source_hashes"][source]:
                raise RuntimeError("SOURCE_BINDING_MISMATCH:" + source)
    return root


def _lease_deadline(ledger_path, lease_id, pending, concurrency, worst_seconds=None):
    with locked_ledger(ledger_path) as data:
        if data.get("schema") != "host-lease-v1":
            raise RuntimeError("LEASE_LEDGER_SCHEMA_MISMATCH")
        matches = [row for row in data.get("leases", []) if row.get("id") == lease_id]
        if len(matches) != 1:
            raise RuntimeError("LEASE_IDENTITY_MISMATCH")
        lease = matches[0]
        now = time.time()
        if lease.get("host") != platform.node() or lease.get("phase") != "P1" or lease.get("boot_id") != boot_id() or not lease.get("persistent") or "end" in lease:
            raise RuntimeError("LEASE_IDENTITY_MISMATCH")
        if lease.get("deadline", 0) <= now or lease.get("start", now) > now:
            raise RuntimeError("LEASE_EXPIRED")
        used = usage(data, now)
        held = sum(max(0.0, row["deadline"] - now) for row in data["leases"] if "end" not in row)
        held_p1 = sum(max(0.0, row["deadline"] - now) for row in data["leases"] if "end" not in row and row["phase"] == "P1")
        if sum(used.values()) + held > CAP_SECONDS or used["P1"] + held_p1 > PHASE_SECONDS["P1"]:
            raise RuntimeError("LEASE_CAP_EXCEEDED")
        worst = (math.ceil(pending / concurrency) * TIMEOUT_SECONDS if worst_seconds is None else worst_seconds) + CLEANUP_SECONDS
        if lease["deadline"] - now < worst:
            raise RuntimeError("LEASE_REMAINING_INSUFFICIENT")
        return lease["deadline"] - CLEANUP_SECONDS


def _env():
    env = {"LANG": "C", "LC_ALL": "C", "RUST_TEST_THREADS": "1"}
    for key in ("PATH", "LD_LIBRARY_PATH"):
        if key in os.environ:
            env[key] = os.environ[key]
    return env


def _atomic_bytes(path, data):
    temporary = path.with_name(path.name + ".tmp-" + str(os.getpid()))
    temporary.write_bytes(data)
    os.replace(temporary, path)


def _call(command, env, folder, label, deadline, started, allocation=None, tracker=None):
    remaining = min(TIMEOUT_SECONDS - (time.monotonic() - started), deadline - time.time())
    stdout = stderr = b""
    exit_code = None
    timed_out = False
    error = None
    tick = time.monotonic()
    if remaining <= 0:
        timed_out = True
    else:
        try:
            if allocation is None:
                process = subprocess.Popen(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
                try:
                    stdout, stderr = process.communicate(timeout=remaining)
                    exit_code = process.returncode
                except subprocess.TimeoutExpired:
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    stdout, stderr = process.communicate()
                    exit_code = process.returncode
                    timed_out = True
            else:
                process = hpc_resources.run_process(command, allocation=allocation, tracker=tracker,
                    env=env, timeout=remaining, metrics_path=folder/(label+".time"))
                stdout, stderr, exit_code = process["stdout"], process["stderr"], process["exit_code"]
                timed_out = process["timed_out"]
        except OSError as exc:
            error = f"{type(exc).__name__}: {exc}"
    elapsed = time.monotonic() - tick
    _atomic_bytes(folder / (label + ".stdout"), stdout)
    _atomic_bytes(folder / (label + ".stderr"), stderr)
    row = {"command": command, "env": env, "exit_code": exit_code, "timed_out": timed_out,
            "elapsed_seconds": elapsed, "error": error,
            "stdout": label + ".stdout", "stdout_sha256": file_digest(folder / (label + ".stdout")),
            "stderr": label + ".stderr", "stderr_sha256": file_digest(folder / (label + ".stderr"))}
    if allocation is not None and isinstance(locals().get("process"),dict):
        row.update({key:process[key] for key in ("affinity_cpus","peak_rss_bytes","rss_source",
            "rss_samples","affinity_escape","affinity_verified","cgroup_oom_events_delta", "assigned_cpu_steal_fraction")})
        row["launch_command"] = process["command"]
        report = folder/(label+".time")
        if report.is_file():
            row["time_report"] = label+".time"
            row["time_report_sha256"] = file_digest(report)
    return row


def _counts(text):
    matches = _RESULT.findall(text)
    if not matches:
        return None
    status, passed, failed, ignored, measured, filtered = matches[-1]
    return {"result": status, "passed": int(passed), "failed": int(failed),
            "ignored": int(ignored), "measured": int(measured), "filtered_out": int(filtered)}


def _receipt_valid(folder, receipt, entry, manifest_sha):
    if receipt.get("schema") != SCHEMA or receipt.get("id") != entry["id"] or receipt.get("manifest_sha256") != manifest_sha or receipt.get("entry_sha256") != digest(entry):
        return False
    transcript = folder / "receipt.jsonl"
    if not transcript.is_file() or transcript.read_text() != canonical(receipt) + "\n":
        return False
    if receipt.get("runner_stderr"):
        path = folder / receipt["runner_stderr"]
        if receipt["runner_stderr"] != "runner.stderr" or not path.is_file() or file_digest(path) != receipt.get("runner_stderr_sha256"):
            return False
    for phase in receipt.get("phases", []):
        for stream in ("stdout", "stderr", "time_report"):
            name = phase.get(stream)
            if stream == "time_report" and name is None:
                continue
            if not isinstance(name, str) or Path(name).name != name:
                return False
            path = folder / name
            if not path.is_file() or file_digest(path) != phase.get(stream + "_sha256"):
                return False
    return True


def _one(entry, output, manifest_sha, deadline, lease_id, exec_sha, allocation=None, tracker=None):
    folder = output / "jobs" / entry["id"]
    folder.mkdir(parents=True, exist_ok=True)
    exe = Path(entry["executable"])
    receipt = {"schema": SCHEMA, "id": entry["id"], "manifest_sha256": manifest_sha,
               "entry_sha256": digest(entry), "executable_sha256": entry["sha256"],
               "source_hashes": entry["source_hashes"], "exec_sha": exec_sha,
               "lease_id": lease_id, "lease_phase": "P1", "lease_host": platform.node(),
               "status": None,
               "phases": [], "test_names": [], "test_counts": None}
    if not exe.is_file():
        receipt["status"] = "ENVIRONMENT_FAILURE"
        receipt["reason"] = "MISSING_EXECUTABLE"
    elif file_digest(exe) != entry["sha256"]:
        receipt["status"] = "ENVIRONMENT_FAILURE"
        receipt["reason"] = "EXECUTABLE_HASH_MISMATCH"
    elif deadline - time.time() < TIMEOUT_SECONDS:
        receipt["status"] = "BUDGET_TIMEOUT_HOLD"
        receipt["reason"] = "INSUFFICIENT_PER_BINARY_DEADLINE"
    else:
        env = _env()
        if allocation is not None:
            env = hpc_resources.thread_environment(env, allocation, contract=True)
        started = time.monotonic()
        listed = _call([entry["executable"], "--list", "--format", "terse", *entry["argv"]], env, folder, "list", deadline, started, allocation, tracker)
        receipt["phases"].append(listed)
        if listed["timed_out"]:
            receipt["status"] = "TEST_TIMEOUT"
            receipt["reason"] = "LIST_TIMEOUT"
        elif listed["error"] or listed["exit_code"] != 0:
            receipt["status"] = "ENVIRONMENT_FAILURE"
            receipt["reason"] = "LIST_FAILURE"
        else:
            raw = (folder / "list.stdout").read_text(errors="replace")
            names = [match.group(1) for line in raw.splitlines() if (match := _TEST.fullmatch(line.strip()))]
            receipt["test_names"] = names
            if not names:
                receipt["status"] = "ENVIRONMENT_FAILURE"
                receipt["reason"] = "EMPTY_OR_UNPARSEABLE_TEST_LIST"
            elif any("holdout" in name.casefold() for name in names):
                receipt["status"] = "PROTECTED_TEST_REFUSED"
                receipt["reason"] = "HOLDOUT_TEST_NAME"
            else:
                ran = _call([entry["executable"], *entry["argv"], "--format", "terse"], env, folder, "run", deadline, started, allocation, tracker)
                receipt["phases"].append(ran)
                if ran["timed_out"]:
                    receipt["status"] = "TEST_TIMEOUT"
                    receipt["reason"] = "RUN_TIMEOUT"
                elif ran["error"]:
                    receipt["status"] = "ENVIRONMENT_FAILURE"
                    receipt["reason"] = "EXEC_FAILURE"
                else:
                    counts = _counts((folder / "run.stdout").read_text(errors="replace") + "\n" + (folder / "run.stderr").read_text(errors="replace"))
                    receipt["test_counts"] = counts
                    if ran["exit_code"] != 0:
                        receipt["status"] = "TEST_FAILURE"
                        receipt["reason"] = "NONZERO_EXIT"
                    elif counts is None or counts["passed"] + counts["failed"] + counts["ignored"] + counts["measured"] != len(names) or counts["filtered_out"] != 0:
                        receipt["status"] = "ENVIRONMENT_FAILURE"
                        receipt["reason"] = "TEST_COUNT_MISMATCH"
                    elif counts["failed"] or counts["result"] != "ok":
                        receipt["status"] = "TEST_FAILURE"
                        receipt["reason"] = "REPORTED_FAILURE"
                    else:
                        receipt["status"] = "OK"
                        receipt["reason"] = "ALL_LISTED_TESTS_ACCOUNTED"
    atomic_json(folder / "receipt.json", receipt)
    _atomic_bytes(folder / "receipt.jsonl", (canonical(receipt) + "\n").encode())
    return receipt


def _safe_one(entry, output, manifest_sha, deadline, lease_id, exec_sha, allocation=None, tracker=None):
    try:
        return _one(entry, output, manifest_sha, deadline, lease_id, exec_sha, allocation, tracker)
    except Exception as exc:
        folder = output / "jobs" / entry["id"]
        folder.mkdir(parents=True, exist_ok=True)
        error_path = folder / "runner.stderr"
        _atomic_bytes(error_path, (f"{type(exc).__name__}: {exc}\n").encode())
        receipt = {"schema": SCHEMA, "id": entry["id"], "manifest_sha256": manifest_sha,
                   "entry_sha256": digest(entry), "executable_sha256": entry["sha256"],
                   "source_hashes": entry["source_hashes"], "exec_sha": exec_sha,
                   "lease_id": lease_id, "lease_phase": "P1", "lease_host": platform.node(),
                   "status": "ENVIRONMENT_FAILURE",
                   "reason": "RUNNER_EXCEPTION", "error_class": type(exc).__name__,
                   "runner_stderr": "runner.stderr", "runner_stderr_sha256": file_digest(error_path),
                   "phases": [], "test_names": [], "test_counts": None}
        atomic_json(folder / "receipt.json", receipt)
        _atomic_bytes(folder / "receipt.jsonl", (canonical(receipt) + "\n").encode())
        return receipt


def run_manifest(manifest_path, output, ledger_path, lease_id, concurrency=4, hpc=False):
    if type(concurrency) is not int or not 1 <= concurrency <= (56 if hpc else 4):
        raise ValueError("concurrency outside permitted range")
    manifest_path = Path(manifest_path)
    manifest = strict_load(manifest_path.read_text())
    entries = _validate_manifest(manifest)
    _check_source(manifest)
    manifest_sha = file_digest(manifest_path)
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    plan = output / "manifest.json"
    if plan.exists() and strict_load(plan.read_text()) != manifest:
        raise RuntimeError("MANIFEST_IDENTITY_MISMATCH")
    if not plan.exists():
        atomic_json(plan, manifest)
    completed, pending = [], []
    for entry in entries:
        folder = output / "jobs" / entry["id"]
        path = folder / "receipt.json"
        if path.exists():
            receipt = strict_load(path.read_text())
            if not _receipt_valid(folder, receipt, entry, manifest_sha):
                raise RuntimeError("RECEIPT_IDENTITY_MISMATCH:" + entry["id"])
            completed.append(receipt)
        else:
            pending.append(entry)
    if not pending:
        return completed
    if hpc:
        resources = hpc_resources.detect_resources()
        cap = None if concurrency == 4 else concurrency
        pool = hpc_resources.ResourcePool(resources,max_workers=cap,telemetry_path=output/"hpc_telemetry.jsonl")
        demand = lambda entry: hpc_resources.demand_for({"id":entry["id"]},kind="contract")
        worst = hpc_resources.estimate_waves(pending,resources,demand=demand,max_workers=pool.max_workers)*TIMEOUT_SECONDS
        deadline = _lease_deadline(ledger_path,lease_id,len(pending),concurrency,worst_seconds=worst)
        completed.extend(pool.run(pending,lambda entry,allocation,tracker:
            _safe_one(entry,output,manifest_sha,deadline,lease_id,manifest["exec_sha"],allocation,tracker),
            demand=demand))
    else:
        deadline = _lease_deadline(ledger_path, lease_id, len(pending), concurrency)
        with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as pool:
            futures = [pool.submit(_safe_one, entry, output, manifest_sha, deadline, lease_id, manifest["exec_sha"]) for entry in pending]
            for future in concurrent.futures.as_completed(futures):
                completed.append(future.result())
    _check_source(manifest)
    for entry in entries:
        path = Path(entry["executable"])
        if path.is_file() and file_digest(path) != entry["sha256"]:
            raise RuntimeError("EXECUTABLE_BINDING_MISMATCH_AFTER_RUN:" + entry["id"])
    return sorted(completed, key=lambda x: x["id"])


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--ledger", required=True)
    parser.add_argument("--lease-id", required=True)
    parser.add_argument("--concurrency", type=int, default=4)
    parser.add_argument("--hpc", action="store_true")
    args = parser.parse_args(argv)
    receipts = run_manifest(args.manifest, args.output, args.ledger, args.lease_id, args.concurrency, args.hpc)
    print(canonical({"schema": SCHEMA, "receipts": len(receipts), "status_counts": {status: sum(x["status"] == status for x in receipts) for status in sorted({x["status"] for x in receipts})}}))


if __name__ == "__main__":
    main()
