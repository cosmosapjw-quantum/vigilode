#!/usr/bin/env python3
"""Finite, resumable case dispatcher; no physics or automatic failed-case retry."""
from __future__ import annotations

import argparse
from concurrent.futures import FIRST_COMPLETED, ProcessPoolExecutor, wait
from contextlib import contextmanager
import fcntl
import hashlib
import importlib
import json
import math
import multiprocessing
import os
from pathlib import Path
import re
import sys
import time
import traceback
import uuid

ROOT = Path(__file__).resolve().parents[1]
MIB = 1024**2


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read_json(path):
    return json.loads(Path(path).read_text())


def atomic_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_name(path.name + "." + uuid.uuid4().hex + ".tmp")
    with temp.open("xb") as handle:
        handle.write(canonical(value) + b"\n")
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temp, path)


@contextmanager
def locked(path):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+b") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        yield


def owner():
    """Record proc-visible identity; never infer ownership from os.getpid()."""
    result = {"token": uuid.uuid4().hex, "created_unix": time.time()}
    try:
        raw = Path("/proc/self/stat").read_text()
        result.update(proc_pid=int(raw.split("(", 1)[0]), start_tick=int(raw.rsplit(")", 1)[1].split()[19]),
                      boot_id=Path("/proc/sys/kernel/random/boot_id").read_text().strip())
        result["pid_namespace"] = os.readlink("/proc/self/ns/pid")
    except (OSError, ValueError, IndexError):
        result["process_identity"] = "unavailable_file_claim_remains_authoritative"
    return result


def safe_path(root, name):
    path = (root / name).resolve()
    path.relative_to(root.resolve())
    return path


def freeze(root, manifest_path):
    manifest = read_json(manifest_path)
    names = manifest.get("identity_files", [])
    if not names:
        raise ValueError("CAMPAIGN identity_files must declare scientific dependencies")
    files = {str(manifest_path.resolve().relative_to(root.resolve())), "provenance/BASELINE_SOURCES.json"}
    for name in names:
        matches = list(root.glob(name))
        if not matches:
            raise ValueError("missing identity input: " + name)
        files.update(str(p.relative_to(root)) for p in matches if p.is_file())
    baseline = read_json(root / "provenance/BASELINE_SOURCES.json")
    for row in baseline["files"]:
        if digest(safe_path(root, row["path"])) != row["sha256"]:
            raise ValueError("baseline byte mismatch: " + row["path"])
        files.add(row["path"])
    file_hashes = {name: digest(safe_path(root, name)) for name in sorted(files)}
    configs, seen = [], set()
    for row in manifest["cases"]:
        config = dict(row["config"])
        case_id = row.get("case_id", config.get("case_id"))
        if not isinstance(case_id, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*", case_id) or case_id in seen:
            raise ValueError("duplicate or unsafe case_id")
        if "case_id" in config and config["case_id"] != case_id:
            raise ValueError("case_id mismatch")
        if any(k.startswith("_") for k in config):
            raise ValueError("reserved config field")
        config["case_id"] = case_id
        canonical(config)
        seen.add(case_id)
        configs.append(config)
    base = hashlib.sha256(canonical(file_hashes)).hexdigest()
    return {"schema": "HHELI_CASE_FREEZE_v1", "base_identity": base, "files": file_hashes, "configs": configs}


def verify_inputs(root, frozen):
    for name, expected in frozen["files"].items():
        if digest(safe_path(root, name)) != expected:
            raise ValueError("scientific input changed: " + name)


def case_identity(frozen, config):
    return hashlib.sha256(canonical({"base": frozen["base_identity"], "config": config})).hexdigest()


def lease_state(receipt):
    """Read-only liveness; no process signals, lease deletion, or claim takeover."""
    try:
        pid, tick, boot = int(receipt["proc_pid"]), int(receipt["start_tick"]), receipt["boot_id"]
        if pid < 1 or tick < 0 or not isinstance(boot, str) or not boot:
            return "unknown"
        if boot != Path("/proc/sys/kernel/random/boot_id").read_text().strip():
            return "dead_other_boot"
        if receipt.get("pid_namespace", os.readlink("/proc/self/ns/pid")) != os.readlink("/proc/self/ns/pid"):
            return "unknown_namespace"
        try:
            stat = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        except FileNotFoundError:
            return "dead_absent_pid"
        if int(stat[19]) != tick:
            return "dead_reused_pid"
        return "dead_zombie" if stat[0] == "Z" else "live"
    except (OSError, ValueError, KeyError, TypeError, IndexError, AttributeError):
        return "unknown"


def valid_completion(directory, identity):
    try:
        done = read_json(directory / "COMPLETE.json")
        result = read_json(directory / "RESULT.json")
        return (done["identity"] == identity == result["scientific_identity_sha256"]
                and result["status"] == "COMPLETE" and done["result_sha256"] == digest(directory / "RESULT.json")
                and bool(done["outputs"]) and all(safe_path(directory, name).stat().st_size == row["bytes"]
                and digest(safe_path(directory, name)) == row["sha256"] for name, row in done["outputs"].items()))
    except (OSError, ValueError, KeyError, TypeError):
        return False


def execute_one(root_text, output_text, frozen, config, module="coupled"):
    root, output = Path(root_text), Path(output_text)
    directory = output / "cases" / config["case_id"]
    identity = case_identity(frozen, config)
    if valid_completion(directory, identity):
        return {"case_id": config["case_id"], "status": "complete_reused", "identity": identity}
    directory.parent.mkdir(parents=True, exist_ok=True)
    try:
        directory.mkdir()  # Atomic single owner, even before the claim receipt exists.
    except FileExistsError:
        return {"case_id": config["case_id"], "status": "existing_preserved", "identity": identity}
    atomic_json(directory / "CLAIM.json", {**owner(), "identity": identity, "config": config})
    try:
        for variable in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "NUMEXPR_NUM_THREADS"):
            os.environ[variable] = "1"
        try:
            os.nice(max(0, 10 - os.nice(0)))
        except (AttributeError, OSError):
            pass
        verify_inputs(root, frozen)
        sys.path.insert(0, str(root / "code"))
        importlib.import_module(module).run_case({**config, "_identity": identity}, directory)
        verify_inputs(root, frozen)
        result = read_json(directory / "RESULT.json")
        if result["status"] != "COMPLETE" or result["scientific_identity_sha256"] != identity:
            raise ValueError("case did not return COMPLETE with the frozen scientific identity")
        outputs = result["outputs"]
        if "trajectory.npz" not in outputs:
            raise ValueError("missing trajectory.npz receipt")
        for name, row in outputs.items():
            path = safe_path(directory, name)
            if path.stat().st_size != row["bytes"] or digest(path) != row["sha256"]:
                raise ValueError("output byte mismatch: " + name)
        atomic_json(directory / "COMPLETE.json", {"identity": identity, "result_sha256": digest(directory / "RESULT.json"), "outputs": outputs})
        return {"case_id": config["case_id"], "status": "completed", "identity": identity}
    except Exception:
        failure = {"case_id": config["case_id"], "status": "failed_preserved", "identity": identity, "traceback": traceback.format_exc()}
        atomic_json(directory / "RUNNER_FAILED.json", failure)
        return failure


def resource_budget(reservation_dirs, requested="auto", cap=None, per_worker_mib=512):
    if per_worker_mib < 1:
        raise ValueError("memory-per-worker-mib must be positive")
    try:
        cores = len(os.sched_getaffinity(0))
    except AttributeError:
        cores = os.cpu_count() or 1
    effective = float(cores)
    cgroup = Path("/sys/fs/cgroup")
    try:
        quota, period = (cgroup / "cpu.max").read_text().split()
        if quota != "max":
            effective = min(effective, int(quota) / int(period))
    except (OSError, ValueError):
        pass
    mem = {r.split(":")[0]: int(r.split()[1]) * 1024 for r in Path("/proc/meminfo").read_text().splitlines()}
    total, available = mem["MemTotal"], mem["MemAvailable"]
    try:
        maximum = int((cgroup / "memory.max").read_text())
        total = min(total, maximum)
        available = min(available, max(0, maximum - int((cgroup / "memory.current").read_text())))
    except (OSError, ValueError):
        pass
    reservations, peer_cpu, peer_memory, unknown = [], 0, 0, False
    for path in sorted({p for directory in reservation_dirs for p in directory.glob("*.json")}):
        try:
            receipt = read_json(path)
            if receipt.get("status") in {"COMPLETED", "RELEASED", "FAILED"}:
                continue
            state = lease_state(receipt)
            reservations.append({"path": str(path), "state": state})
            if state.startswith("dead_"):
                continue
            if state != "live":
                unknown = True
                continue
            n = max(1, int(receipt.get("workers", 1)))
            m = max(per_worker_mib * MIB, int(receipt.get("memory_bytes", n * per_worker_mib * MIB)))
        except (OSError, ValueError, TypeError, AttributeError):
            reservations.append({"path": str(path), "state": "unknown"}); unknown = True
            continue
        peer_cpu += n; peer_memory += m
    if unknown:  # Aggregate unreadable identities reserve one slot; measured load/RAM still apply.
        peer_cpu += 1; peer_memory += per_worker_mib * MIB
    load = max(0.0, os.getloadavg()[0])
    os_memory = max(512 * MIB, min(4096 * MIB, int(total * .1)))
    cpu_slots = max(0, math.floor(effective) - 1 - max(math.ceil(load), peer_cpu))
    memory_slots = max(0, (available - os_memory - peer_memory) // (per_worker_mib * MIB))
    default_cap = 63 if effective >= 32 and total >= 64 * 1024**3 else 4
    desired = default_cap if requested == "auto" else int(requested)
    if desired < 1 or (cap is not None and cap < 1) or per_worker_mib < 1:
        raise ValueError("worker and memory limits must be positive")
    workers = max(0, min(desired, cap or 63, 63, cpu_slots, memory_slots))
    return {"workers": workers, "affinity_cores": cores, "effective_cpu_quota": effective,
            "load_1m": load, "cpu_slots": cpu_slots, "memory_limit_bytes": total, "memory_available_bytes": available,
            "memory_reserve_bytes": os_memory, "memory_per_worker_bytes": per_worker_mib * MIB,
            "peer_reservations": reservations, "peer_reserved_cpu": peer_cpu, "peer_reserved_bytes": peer_memory,
            "default_worker_cap": default_cap, "host_claim": "observed current host; no NCP identity inferred"}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--execute", action="store_true"); mode.add_argument("--plan", action="store_true")
    parser.add_argument("--manifest", type=Path, default=ROOT / "data/CAMPAIGN.json")
    parser.add_argument("--output", type=Path, default=ROOT / "results/pilot")
    parser.add_argument("--workers", default="auto"); parser.add_argument("--max-workers", type=int)
    parser.add_argument("--memory-per-worker-mib", type=int, default=512)
    parser.add_argument("--peer-reservations", action="append", type=Path, default=[])
    parser.add_argument("--limit", type=int); parser.add_argument("--wall-seconds", type=float)
    args = parser.parse_args(argv)
    if args.limit is not None and args.limit < 1 or args.wall_seconds is not None and args.wall_seconds <= 0:
        parser.error("limit and wall-seconds must be positive")
    frozen = freeze(ROOT, args.manifest)
    configs = frozen["configs"][:args.limit]
    reservation_dir = ROOT / "results/.reservations"
    directories = [reservation_dir, *args.peer_reservations]
    def budget():
        return resource_budget(directories, args.workers, args.max_workers, args.memory_per_worker_mib)
    if not args.execute:
        print(json.dumps({"status": "planned", "selected_cases": len(configs), "total_cases": len(frozen["configs"]),
                          "base_identity": frozen["base_identity"], "resources": budget()}, indent=2)); return 0
    args.output.mkdir(parents=True, exist_ok=True)
    with locked(args.output / ".freeze.lock"):
        snapshot = args.output / "FROZEN.json"
        if snapshot.exists() and read_json(snapshot) != frozen:
            raise ValueError("frozen campaign changed; preserve this output and choose a new --output")
        if not snapshot.exists():
            atomic_json(snapshot, frozen)
    job = owner(); lease = reservation_dir / (job["token"] + ".json")
    classified = []
    for config in configs:
        directory, identity = args.output / "cases" / config["case_id"], case_identity(frozen, config)
        if valid_completion(directory, identity):
            status = "complete_reused"
        elif directory.exists():
            status = "existing_preserved"
        else:
            continue
        classified.append({"case_id": config["case_id"], "status": status, "identity": identity})
    existing_ids = {record["case_id"] for record in classified}
    configs = [config for config in configs if config["case_id"] not in existing_ids]
    if not configs:
        success = all(r["status"] == "complete_reused" for r in classified)
        summary = {"records": classified, "unsubmitted": 0, "new_workers": 0, "status": "complete_reused" if success else "existing_preserved"}
        atomic_json(args.output / ("RUN_" + job["token"] + ".json"), summary)
        print(json.dumps(summary, indent=2)); return 0 if success else 2
    with locked(reservation_dir / ".allocation.lock"):
        resources = budget(); workers = min(resources["workers"], len(configs))
        if workers:
            atomic_json(lease, {**job, "status": "RUNNING", "workers": workers,
                               "memory_bytes": workers * resources["memory_per_worker_bytes"]})
    if not workers:
        print(json.dumps({"status": "resource_deferred", "resources": resources}, indent=2)); return 3
    started, records, pending, index = time.monotonic(), classified, {}, 0
    try:
        with ProcessPoolExecutor(max_workers=workers, mp_context=multiprocessing.get_context("spawn")) as pool:
            while index < len(configs) or pending:
                within_budget = args.wall_seconds is None or time.monotonic() - started < args.wall_seconds
                while within_budget and index < len(configs) and len(pending) < workers:
                    config = configs[index]; index += 1
                    pending[pool.submit(execute_one, str(ROOT), str(args.output), frozen, config)] = config["case_id"]
                if not pending:
                    break
                finished, _ = wait(pending, timeout=1, return_when=FIRST_COMPLETED)
                for future in finished:
                    case_id = pending.pop(future)
                    try:
                        records.append(future.result())
                    except Exception:
                        records.append({"case_id": case_id, "status": "worker_failed_preserved", "traceback": traceback.format_exc()})
                    atomic_json(args.output / ("RUN_" + job["token"] + ".json"), {"resources": resources, "records": records})
    finally:
        atomic_json(lease, {**job, "status": "RELEASED", "workers": workers})
    summary = {"records": records, "unsubmitted": len(configs) - index, "resources": resources, "wall_seconds": time.monotonic() - started}
    atomic_json(args.output / ("RUN_" + job["token"] + ".json"), summary)
    print(json.dumps(summary, indent=2))
    return 0 if not summary["unsubmitted"] and all(r["status"] in {"completed", "complete_reused"} for r in records) else 2


if __name__ == "__main__":
    raise SystemExit(main())
