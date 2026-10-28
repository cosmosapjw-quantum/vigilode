#!/usr/bin/env python3
"""Source-pinned, append-only local ODE timing and profiling campaigns.

Python standard library only. This controller never installs software, changes
system settings, runs two benchmark processes concurrently, or contacts NCP.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import datetime as dt
import hashlib
import importlib.metadata
import json
import math
import os
from pathlib import Path
import platform
import resource
import shutil
import signal
import statistics
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
METHODS = ("dopri5", "rodas5p-mf", "rodas5p-dense", "switching")
THREADS = (1, 2, 4, 8, 12)
SMALL_CASES = {1, 2, 3, 5, 7, 8, 10, 15, 16}


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def json_new(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x") as f:
        json.dump(value, f, indent=2, allow_nan=False)
        f.write("\n")


def utc():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def source_identity(repo, expected_commit=None, expected_tree=None, allow_dirty=False):
    def git(*args):
        return subprocess.check_output(["git", "-C", str(repo), *args], text=True).strip()
    identity = {"commit": git("rev-parse", "HEAD"), "tree": git("rev-parse", "HEAD^{tree}"),
                "status_porcelain": git("status", "--porcelain", "--untracked-files=all")}
    identity["clean"] = not identity["status_porcelain"]
    identity["exploratory_dirty_run"] = not identity["clean"]
    if expected_commit and identity["commit"] != expected_commit:
        raise ValueError("checkout commit differs from expected commit; use a separate exact worktree")
    if expected_tree and identity["tree"] != expected_tree:
        raise ValueError("checkout tree differs from expected tree")
    if not identity["clean"] and not allow_dirty:
        raise ValueError("checkout is dirty; existing checkout is preserved, prepare a separate worktree")
    return identity


def validate_build_receipt(receipt, source, binary):
    if receipt.get("exit_code") != 0 or receipt.get("source") != source or receipt.get("binary_sha256") != digest(binary):
        raise ValueError("build receipt does not bind this successful binary to this source")
    return receipt


def topology():
    available = sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else list(range(os.cpu_count() or 1))
    cores = {}
    rows = []
    for cpu in available:
        base = Path(f"/sys/devices/system/cpu/cpu{cpu}")
        def read(rel, fallback):
            try:
                return (base / rel).read_text().strip()
            except OSError:
                return fallback
        package = read("topology/physical_package_id", "unknown")
        core = read("topology/core_id", str(cpu))
        key = (package, core)
        cores.setdefault(key, cpu)
        rows.append({"cpu": cpu, "package": package, "core": core,
                     "thread_siblings": read("topology/thread_siblings_list", str(cpu)),
                     "l3_shared_cpus": read("cache/index3/shared_cpu_list", "unknown"),
                     "governor": read("cpufreq/scaling_governor", "unknown")})
    return {"allowed_logical_cpus": available, "physical_cpu_selection": list(cores.values()),
            "physical_core_count": len(cores), "affinity_supported": hasattr(os, "sched_setaffinity"), "cpu_rows": rows}


def checked_json_lines(path):
    rows, errors = [], []
    for index, line in enumerate(Path(path).read_text(errors="replace").splitlines(), 1):
        if not line.strip():
            continue
        try:
            rows.append(json.loads(line, parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value))))
        except (ValueError, json.JSONDecodeError) as e:
            errors.append({"line": index, "error": str(e)})
    return rows, errors


def capabilities(probe_perf=True):
    packages = {}
    for name in ("numpy", "scipy", "matplotlib"):
        try:
            packages[name] = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            packages[name] = None
    perf = {"path": shutil.which("perf"), "usable": False, "probe": "NOT_RUN"}
    if perf["path"] and probe_perf:
        r = subprocess.run([perf["path"], "stat", "-e", "task-clock", "--", "true"],
                           capture_output=True, text=True, timeout=10)
        perf.update(usable=r.returncode == 0, probe="COMPLETED", exit_code=r.returncode,
                    stderr=r.stderr)
    return {"python": sys.version, "platform": platform.platform(), "hostname": platform.node(),
            "cpu_topology": topology(), "packages": packages, "perf": perf,
            "gnu_time": "/usr/bin/time" if Path("/usr/bin/time").exists() else None,
            "julia": shutil.which("julia"), "gfortran": shutil.which("gfortran"),
            "ncp_used": False,
            "external_comparators": {
                "vode": {"available": packages["scipy"] is not None, "adapter": "SciPy wrapper of double-precision VODE; Python callback cost included"},
                "julia": {"executable_available": shutil.which("julia") is not None, "packages_verified": False},
                "diffsol": {"available": False, "reason": "requires explicit built adapter path and matched model verification"}}}


def run_process(command, folder, timeout, memory_gib, cpus, environment=None, expect_result=True):
    """Keep all streams and process-group timeout failures; never overwrite."""
    folder = Path(folder)
    folder.mkdir(parents=True, exist_ok=False)
    started = utc()
    env = os.environ.copy()
    env.update(environment or {})
    def limits():
        if cpus and hasattr(os, "sched_setaffinity"):
            os.sched_setaffinity(0, cpus)
        if memory_gib:
            amount = int(memory_gib * (1 << 30))
            resource.setrlimit(resource.RLIMIT_AS, (amount, amount))
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    wrapped = list(map(str, command))
    gnu_time = Path("/usr/bin/time")
    if gnu_time.exists():
        wrapped = [str(gnu_time), "-v", "-o", str((folder / "resource.txt").resolve()), "--", *wrapped]
    initial = {"schema_version": 1, "started_utc": started, "command": list(map(str, command)),
               "wrapped_command": wrapped, "timeout_seconds": timeout, "address_space_limit_gib": memory_gib,
               "cpus": cpus, "thread_environment": environment or {}}
    json_new(folder / "START.json", initial)
    start = time.monotonic()
    timed_out, spawn_error = False, None
    with (folder / "stdout.jsonl").open("xb") as out, (folder / "stderr.log").open("xb") as err:
        try:
            process = subprocess.Popen(wrapped, stdout=out, stderr=err, env=env,
                                       preexec_fn=limits, start_new_session=True)
            try:
                code = process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
                code = 124
        except (OSError, subprocess.SubprocessError) as e:
            spawn_error = repr(e)
            code = 127
    seconds = time.monotonic() - start
    rows, parse_errors = checked_json_lines(folder / "stdout.jsonl")
    result_rows = [r for r in rows if isinstance(r, dict) and r.get("kind") == "result"]
    result = result_rows[-1] if len(result_rows) == 1 else None
    category = ("TIMEOUT" if timed_out else "SPAWN_FAILURE" if spawn_error else
                "PROCESS_FAILURE" if code else "MALFORMED_RESULT" if parse_errors or (expect_result and len(result_rows) != 1) else "COMPLETED")
    receipt = {**initial, "finished_utc": utc(), "elapsed_process_seconds": seconds,
               "exit_code": code, "timed_out": timed_out, "spawn_error": spawn_error,
               "execution_status": category, "parse_errors": parse_errors,
               "result": result, "stdout_sha256": digest(folder / "stdout.jsonl"),
               "stderr_sha256": digest(folder / "stderr.log")}
    if (folder / "resource.txt").exists():
        receipt["resource_sha256"] = digest(folder / "resource.txt")
    return receipt


def validate_config(config):
    if config.get("size") not in ("smoke", "standard"):
        raise ValueError("size must be smoke or standard")
    if not config.get("cases") or any(not isinstance(i, int) or i not in range(1, 25) for i in config["cases"]):
        raise ValueError("cases must be nonempty case IDs 1..24")
    if len(set(config["cases"])) != len(config["cases"]):
        raise ValueError("duplicate cases")
    if not config.get("threads") or any(i not in THREADS for i in config["threads"]):
        raise ValueError("registered thread levels are 1,2,4,8,12")
    if not config.get("methods") or any(m not in METHODS for m in config["methods"]):
        raise ValueError("unknown solver method")
    if len(set(config["threads"])) != len(config["threads"]) or len(set(config["methods"])) != len(config["methods"]):
        raise ValueError("duplicate threads or methods")
    for key in ("rtol", "atol", "per_run_seconds", "campaign_seconds", "memory_gib"):
        if not isinstance(config.get(key), (int, float)) or not math.isfinite(config[key]) or config[key] <= 0:
            raise ValueError(f"{key} must be positive finite")
    for key in ("repeats", "max_steps", "ensemble_count"):
        if not isinstance(config.get(key), int) or config[key] <= 0:
            raise ValueError(f"{key} must be positive integer")
    if config.get("profiling") not in ("none", "callbacks", "perf-stat", "perf-record"):
        raise ValueError("unknown profiling mode")
    if any(not isinstance(x, (int, float)) or not math.isfinite(x) or not 0 < x <= 1
           for x in config.get("accuracy_fractions", [])):
        raise ValueError("accuracy probe fractions must lie in (0,1]")
    if len(set(config.get("accuracy_fractions", []))) != len(config.get("accuracy_fractions", [])):
        raise ValueError("duplicate accuracy probe fractions")
    return config


def make_plan(config, catalog, source, binary, host):
    validate_config(config)
    available = {int(x["id"]): x for x in catalog["cases"]}
    physical = host["cpu_topology"]["physical_cpu_selection"]
    tasks, skipped = [], []
    for case in config["cases"]:
        if case not in available:
            raise ValueError(f"catalog missing case {case}")
        item = available[case]
        methods = [m for m in config["methods"] if m in item.get("supported_methods", METHODS)]
        dimension = item.get("dimension_" + config["size"], item.get("dimension", 0))
        if dimension > item.get("dense_dimension_limit", 256):
            methods = [m for m in methods if m != "rodas5p-dense"]
        layer = "ensemble" if case in SMALL_CASES else "data"
        # The serial baseline is a separate latency experiment; ensemble/data
        # scaling uses its own one-thread baseline with the SAME work quantity.
        variants = [("serial", 1)] + [(layer, t) for t in config["threads"]]
        for method in config["methods"]:
            if method not in methods:
                skipped.append({"case": case, "method": method, "reason": "catalog method unsupported or dense dimension exceeds declared cap"})
        for repeat in range(0 if config.get("accuracy_only") else config["repeats"]):
            order = methods[repeat % len(methods):] + methods[:repeat % len(methods)] if methods else []
            for layer_name, threads in variants:
                for method in order:
                    key = {"case": case, "method": method, "parallel_layer": layer_name,
                           "threads": threads, "repeat": repeat, "instrumented": False}
                    if threads > len(physical):
                        skipped.append({**key, "reason": "insufficient admitted physical cores"})
                        continue
                    tasks.append({**key, "cpus": physical[:threads], "id": hashlib.sha256(canonical(key).encode()).hexdigest()[:16]})
        for fraction in config.get("accuracy_fractions", []):
            end = item.get("t_end")
            if not isinstance(end, (int, float)) or not math.isfinite(end) or end <= 0:
                raise ValueError(f"case {case} needs its described t_end for accuracy probes")
            for method in methods:
                key = {"case": case, "method": method, "parallel_layer": "serial", "threads": 1,
                       "repeat": 0, "instrumented": False, "purpose": "accuracy-probe",
                       "end_time": end * fraction, "end_fraction": fraction}
                tasks.append({**key, "cpus": physical[:1], "id": hashlib.sha256(canonical(key).encode()).hexdigest()[:16]})
        if config["profiling"] != "none":
            for method in methods:
                key = {"case": case, "method": method, "parallel_layer": "serial", "threads": 1,
                       "repeat": 0, "instrumented": True}
                tasks.append({**key, "cpus": physical[:1], "id": hashlib.sha256(canonical(key).encode()).hexdigest()[:16]})
    # Interleave problems by replicate, retaining deterministic cyclic method
    # ordering. Profiles follow clean timing runs and never enter speed ratios.
    tasks.sort(key=lambda x: (x["instrumented"], x.get("purpose") == "accuracy-probe", x["repeat"]))
    return {"schema_version": 1, "created_utc": utc(), "source": source, "binary": str(Path(binary).resolve()),
            "binary_sha256": digest(binary), "config": config, "catalog": catalog,
            "host": host, "tasks": tasks, "planned_skips": skipped,
            "nominal_case_thread_configurations": len(config["cases"]) * len(config["threads"]),
            "planned_processes": len(tasks), "per_process_timeout_worst_seconds": len(tasks) * config["per_run_seconds"],
            "cumulative_campaign_budget_seconds": config["campaign_seconds"],
            "budget_accounting": "prior completed process wall time plus full timeout for interrupted attempts plus current invocation wall time; earlier controller overhead is not reconstructed",
            "timing_policy": "one process at a time; independent process repetitions; cyclic method order",
            "parallelism_policy": "serial latency, data scaling and independent ensemble throughput are separate; no nested solver/data pools",
            "ncp_used": False}


def task_command(plan, task):
    c = plan["config"]
    cmd = [plan["binary"], "--case", str(task["case"]), "--size", c["size"],
           "--method", task["method"], "--parallel-layer", task["parallel_layer"],
           "--threads", str(task["threads"]), "--rtol", str(c["rtol"]), "--atol", str(c["atol"]),
           "--max-steps", str(c["max_steps"]), "--ensemble-count", str(c["ensemble_count"])]
    if c.get("emit_state", False):
        cmd.append("--emit-state")
    if task["instrumented"]:
        cmd.append("--instrument")
    if "end_time" in task:
        cmd += ["--end", str(task["end_time"])]
    return cmd


def eligible(receipt):
    r = receipt.get("result") or {}
    error = r.get("actual_error")
    limit = r.get("error_limit")
    return bool(receipt.get("execution_status") == "COMPLETED" and receipt.get("exit_code") == 0
                and not receipt.get("task", {}).get("instrumented")
                and receipt.get("task", {}).get("purpose") != "accuracy-probe"
                and not receipt.get("source", {}).get("exploratory_dirty_run")
                and r.get("speed_comparison_eligible") is True
                and r.get("scientific_status") in ("PASS_SCOPED", "PASS")
                and isinstance(error, (int, float)) and math.isfinite(error)
                and isinstance(limit, (int, float)) and math.isfinite(limit) and 0 <= error <= limit)


def result_contract_errors(result, task):
    if result is None:
        return []
    errors = []
    for name in ("method", "parallel_layer", "threads"):
        if result.get(name) != task[name]:
            errors.append(f"result {name} differs from requested task")
    actual_case = result.get("case_id", result.get("case"))
    if str(actual_case) != str(task["case"]):
        errors.append("result case differs from requested task")
    return errors


def verify_receipt(path, source, binary_hash, task=None):
    path = Path(path)
    receipt = json.loads(path.read_text())
    if receipt.get("source") != source or receipt.get("binary_sha256") != binary_hash:
        raise ValueError("receipt source/binary identity mismatch: " + str(path))
    if task is not None and receipt.get("task") != task:
        raise ValueError("receipt task identity mismatch: " + str(path))
    for stream in ("stdout", "stderr"):
        name = "stdout.jsonl" if stream == "stdout" else "stderr.log"
        if digest(path.parent / name) != receipt.get(stream + "_sha256"):
            raise ValueError("receipt raw stream hash mismatch: " + str(path))
    parsed, errors = checked_json_lines(path.parent / "stdout.jsonl")
    results = [row for row in parsed if isinstance(row, dict) and row.get("kind") == "result"]
    actual = results[0] if len(results) == 1 else None
    if "samples" in receipt and receipt["samples"] != [row for row in parsed if isinstance(row, dict) and row.get("kind") == "sample"]:
        raise ValueError("receipt stage samples differ from raw stream: " + str(path))
    if receipt.get("result") != actual:
        raise ValueError("receipt result differs from raw stream: " + str(path))
    return receipt


def run_campaign(plan_path, output, retry_failed=False):
    plan_path, output = Path(plan_path), Path(output)
    plan = json.loads(plan_path.read_text())
    validate_config(plan["config"])
    current = source_identity(REPO, plan["source"]["commit"], plan["source"]["tree"],
                              plan["source"].get("exploratory_dirty_run", False))
    if canonical(current) != canonical(plan["source"]):
        raise ValueError("source worktree status changed after plan")
    if "hostname" in plan["host"]:
        current_host = capabilities(probe_perf=False)
        if current_host["hostname"] != plan["host"]["hostname"] or current_host["platform"] != plan["host"]["platform"] or current_host["cpu_topology"]["allowed_logical_cpus"] != plan["host"]["cpu_topology"]["allowed_logical_cpus"] or current_host["cpu_topology"]["physical_cpu_selection"] != plan["host"]["cpu_topology"]["physical_cpu_selection"]:
            raise ValueError("host or CPU affinity changed after planning; create a new plan/output")
    if digest(plan["binary"]) != plan["binary_sha256"]:
        raise ValueError("binary changed after plan")
    output.mkdir(parents=True, exist_ok=True)
    identity = {"plan_sha256": digest(plan_path), "binary_sha256": plan["binary_sha256"], "source": plan["source"],
                "expected_repeats": plan["config"]["repeats"], "planned_processes": plan["planned_processes"],
                "tasks": {task["id"]: task for task in plan["tasks"]}}
    lock = output / "CAMPAIGN_IDENTITY.json"
    if lock.exists():
        if json.loads(lock.read_text()) != identity:
            raise ValueError("output belongs to a different campaign")
    else:
        json_new(lock, identity)
    cumulative = sum(json.loads(p.read_text()).get("elapsed_process_seconds", 0) for p in output.glob("runs/*/attempt_*/RECEIPT.json"))
    # Interrupted attempts have no reliable finish time; conservatively charge their full timeout.
    interrupted = [p for p in output.glob("runs/*/attempt_*/START.json") if not (p.parent / "RECEIPT.json").exists()]
    cumulative += sum(json.loads(p.read_text()).get("timeout_seconds", 0) for p in interrupted)
    this_started = time.monotonic()
    pending, executed, resumed = [], 0, 0
    for task in plan["tasks"]:
        root = output / "runs" / task["id"]
        prior = sorted(root.glob("attempt_*/RECEIPT.json")) if root.exists() else []
        if prior:
            last = verify_receipt(prior[-1], plan["source"], plan["binary_sha256"], task)
            if not retry_failed or last["execution_status"] == "COMPLETED":
                resumed += 1
                continue
        remaining = plan["config"]["campaign_seconds"] - cumulative - (time.monotonic() - this_started)
        if remaining <= 0:
            pending.append({"task_id": task["id"], "reason": "accounted process budget exhausted"})
            continue
        attempt = 1 + max((int(p.name.split("_")[1]) for p in root.glob("attempt_*") if p.name.split("_")[1].isdigit()), default=0) if root.exists() else 1
        folder = root / f"attempt_{attempt:03d}"
        cmd = task_command(plan, task)
        profile = {"requested": plan["config"]["profiling"] if task["instrumented"] else "none", "status": "NOT_REQUESTED"}
        perf = plan["host"]["perf"]
        if task["instrumented"]:
            profile["status"] = "CALLBACK_INSTRUMENTATION"
            if plan["config"]["profiling"].startswith("perf-"):
                if perf.get("usable"):
                    if plan["config"]["profiling"] == "perf-stat":
                        cmd = [perf["path"], "stat", "-x", ",", "-e", "task-clock,cycles,instructions,cache-misses,branches,branch-misses", "--", *cmd]
                    else:
                        # perf.data is written only under this attempt directory.
                        cmd = [perf["path"], "record", "-g", "-o", str((folder / "perf.data").resolve()), "--", *cmd]
                    profile["status"] = "PERF_ATTEMPTED"
                else:
                    profile["status"] = "PERF_UNAVAILABLE_CALLBACKS_ONLY"
        env = {"RAYON_NUM_THREADS": str(task["threads"]), "OMP_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1",
               "MKL_NUM_THREADS": "1", "BLIS_NUM_THREADS": "1", "VECLIB_MAXIMUM_THREADS": "1",
               "VIGILODE_SOURCE_COMMIT": plan["source"]["commit"]}
        receipt = run_process(cmd, folder, min(remaining, plan["config"]["per_run_seconds"]),
                              plan["config"]["memory_gib"], task["cpus"], env)
        receipt.update(task=task, source=plan["source"], binary_sha256=plan["binary_sha256"], profile=profile)
        receipt["result_contract_errors"] = result_contract_errors(receipt.get("result"), task)
        if receipt["result_contract_errors"] and receipt["execution_status"] == "COMPLETED":
            receipt["execution_status"] = "RESULT_CONTRACT_MISMATCH"
        receipt["eligible_for_timing_comparison"] = eligible(receipt)
        json_new(folder / "RECEIPT.json", receipt)
        executed += 1
        print(canonical({"task": task["id"], "execution": receipt["execution_status"],
                         "scientific": (receipt.get("result") or {}).get("scientific_status"),
                         "seconds": receipt["elapsed_process_seconds"]}), flush=True)
    summary = {"schema_version": 1, "finished_utc": utc(), "executed_now": executed, "resumed_existing": resumed,
               "unexecuted": pending, "planned_skips": plan["planned_skips"],
               "cumulative_previous_process_seconds": cumulative, "this_invocation_wall_seconds": time.monotonic() - this_started}
    json_new(output / f"EXECUTION_{time.time_ns()}.json", summary)
    return summary


def analyze(output):
    output = Path(output)
    identity = json.loads((output / "CAMPAIGN_IDENTITY.json").read_text())
    receipts = []
    for folder in sorted((output / "runs").glob("*")):
        attempts = sorted(folder.glob("attempt_*/RECEIPT.json"))
        if attempts:
            if folder.name not in identity["tasks"]:
                raise ValueError("unexpected task folder: " + str(folder))
            r = verify_receipt(attempts[-1], identity["source"], identity["binary_sha256"], identity["tasks"][folder.name])
            r["receipt_path"] = str(attempts[-1].relative_to(output))
            receipts.append(r)
    groups, profiles, accuracy_probes = defaultdict(list), [], []
    for r in receipts:
        task = r["task"]
        result = r.get("result") or {}
        if task.get("purpose") == "accuracy-probe":
            accuracy_probes.append({"task": task, "receipt": r["receipt_path"],
                                    "execution_status": r["execution_status"], "result": result})
            continue
        if task["instrumented"]:
            profiles.append({"task": task, "receipt": r["receipt_path"], "profile": r.get("profile"),
                             "phase_seconds": result.get("phase_seconds"), "counters": result.get("counters"),
                             "inference": "instrumented diagnostic; callback times may overlap, so do not subtract blindly from total"})
        key = (task["case"], task["method"], task["parallel_layer"], task["threads"])
        if not task["instrumented"]:
            groups[key].append(r)
    rows = []
    for key, values in sorted(groups.items()):
        good = [x for x in values if eligible(x)]
        seconds = [(x["result"].get("elapsed_seconds")) for x in good]
        seconds = [s for s in seconds if isinstance(s, (float, int)) and math.isfinite(s) and s > 0]
        all_good = len(good) == len(values) == len(seconds) == identity["expected_repeats"]
        rows.append({"case": key[0], "method": key[1], "parallel_layer": key[2], "threads": key[3],
                     "samples": len(values), "expected_samples": identity["expected_repeats"],
                     "eligible_samples": len(good), "complete_eligible_group": all_good,
                     "seconds_median": statistics.median(seconds) if all_good and seconds else None,
                     "seconds_min": min(seconds) if all_good and seconds else None,
                     "seconds_max": max(seconds) if all_good and seconds else None,
                     "execution_statuses": dict(Counter(x["execution_status"] for x in values)),
                     "scientific_statuses": dict(Counter((x.get("result") or {}).get("scientific_status", "NO_RESULT") for x in values)),
                     "receipts": [x["receipt_path"] for x in values]})
    baseline = {(r["case"], r["method"], r["parallel_layer"]): r for r in rows if r["threads"] == 1}
    for row in rows:
        one = baseline.get((row["case"], row["method"], row["parallel_layer"]))
        row["same_layer_speedup"] = one["seconds_median"] / row["seconds_median"] if one and one["seconds_median"] and row["seconds_median"] else None
        row["parallel_efficiency"] = row["same_layer_speedup"] / row["threads"] if row["same_layer_speedup"] else None
    summary = {"schema_version": 1, "created_utc": utc(), "groups": rows, "profiles": profiles,
               "accuracy_probes": accuracy_probes,
               "retained_scientific_failure_or_unvalidated_receipts": [x["receipt_path"] for x in receipts if (x.get("result") or {}).get("scientific_status") not in ("PASS_SCOPED", "PASS")],
               "retained_failure_receipts": [x["receipt_path"] for x in receipts if x["execution_status"] != "COMPLETED"],
               "ranking_scope": "same case, actual-error criterion, work quantity, output contract and parallel layer only",
               "accuracy_scope": "endpoint/reference sampling; accuracy probes are separate cold solves, not one trajectory or a continuous-time certificate; wrapped endpoint phase cannot exclude missed cycles",
               "bottleneck_questions": ["RHS/JVP fraction", "linear solves/projections/factorization work", "rejected/pilot/fallback work", "memory/cache pressure", "pool setup and synchronization", "unattributed controller/output work"],
               "broad_production": "UNRESOLVED_BY_THIS_CAMPAIGN_ALONE"}
    path = output / f"ANALYSIS_{time.time_ns()}.json"
    json_new(path, summary)
    md = ["# Local bottleneck campaign", "", "Only accuracy-eligible uninstrumented rows enter timing ratios. Profiling samples are separate.", "",
          "| Case | Method | Layer | Threads | Samples | Median seconds | Same-layer speedup |", "|---|---|---|---:|---:|---:|---:|"]
    for r in rows:
        fmt = lambda x: "—" if x is None else f"{x:.5g}"
        md.append(f"| {r['case']} | {r['method']} | {r['parallel_layer']} | {r['threads']} | {r['eligible_samples']}/{r['samples']} | {fmt(r['seconds_median'])} | {fmt(r['same_layer_speedup'])} |")
    path.with_suffix(".md").write_text("\n".join(md) + "\n")
    return {"analysis": str(path), "groups": len(rows), "profile_rows": len(profiles)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    pre = sub.add_parser("preflight")
    pre.add_argument("--output", required=True, type=Path)
    pre.add_argument("--no-perf-probe", action="store_true")
    pl = sub.add_parser("plan")
    pl.add_argument("--config", required=True, type=Path)
    pl.add_argument("--binary", required=True, type=Path)
    pl.add_argument("--output", required=True, type=Path)
    pl.add_argument("--expected-commit")
    pl.add_argument("--expected-tree")
    pl.add_argument("--allow-dirty-smoke", action="store_true")
    pl.add_argument("--build-receipt", type=Path)
    run = sub.add_parser("run")
    run.add_argument("--plan", required=True, type=Path)
    run.add_argument("--output", required=True, type=Path)
    run.add_argument("--retry-failed", action="store_true")
    an = sub.add_parser("analyze")
    an.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.action == "preflight":
        host = capabilities(not args.no_perf_probe)
        json_new(args.output, host)
        print(canonical({"preflight": str(args.output), "physical_cores": host["cpu_topology"]["physical_core_count"]}))
    elif args.action == "plan":
        config = validate_config(json.loads(args.config.read_text()))
        if args.allow_dirty_smoke and config["size"] != "smoke":
            raise ValueError("dirty execution is restricted to explicitly exploratory smoke")
        if not args.allow_dirty_smoke and (not args.expected_commit or not args.expected_tree):
            raise ValueError("published campaigns require both expected commit and tree")
        source = source_identity(REPO, args.expected_commit, args.expected_tree, args.allow_dirty_smoke)
        build_receipt = None
        if args.build_receipt:
            build_receipt = json.loads(args.build_receipt.read_text())
            validate_build_receipt(build_receipt, source, args.binary)
        elif not args.allow_dirty_smoke:
            raise ValueError("published campaigns require --build-receipt from build_local.py")
        cat = subprocess.run([str(args.binary.resolve()), "--list"], capture_output=True, text=True, timeout=30, check=True)
        catalog = json.loads(cat.stdout)
        if config.get("accuracy_fractions"):
            for item in catalog["cases"]:
                if int(item["id"]) in config["cases"]:
                    d = subprocess.run([str(args.binary.resolve()), "--describe", "--case", str(item["id"]), "--size", config["size"]], capture_output=True, text=True, timeout=30, check=True)
                    descriptor = json.loads(d.stdout)
                    item["t_end"] = descriptor.get("t_end", descriptor.get("model_contract", {}).get("t_end"))
                    item["descriptor_sha256"] = hashlib.sha256(d.stdout.encode()).hexdigest()
        plan = make_plan(config, catalog, source, args.binary, capabilities())
        plan["build_receipt_sha256"] = digest(args.build_receipt) if args.build_receipt else None
        plan["build_receipt"] = build_receipt
        json_new(args.output, plan)
        print(canonical({"plan": str(args.output), "processes": plan["planned_processes"], "skips": len(plan["planned_skips"]),
                         "worst_per_run_budget_seconds": plan["per_process_timeout_worst_seconds"],
                         "campaign_budget_seconds": plan["cumulative_campaign_budget_seconds"]}))
    elif args.action == "run":
        print(canonical(run_campaign(args.plan, args.output, args.retry_failed)))
    else:
        print(canonical(analyze(args.output)))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as e:
        print(canonical({"status": "BLOCKED", "error": str(e)}), file=sys.stderr)
        raise SystemExit(2)
