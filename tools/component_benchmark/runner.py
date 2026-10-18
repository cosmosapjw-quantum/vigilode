"""Isolated adapter execution with atomic job receipts and bounded host leases."""
from __future__ import annotations

import concurrent.futures
import json
import os
import platform
import random
import subprocess
import time
from pathlib import Path

from . import SCHEMA
from . import hpc as hpc_resources
from .budget import close, reserve, locked_ledger
from .protocol import canonical, digest, file_digest, strict_load, validate_case, validate_result

TIMING_SCOPE = "warmed_solver_setup_integration_output_certificate"
FIXED_SOLVERS = {"native-bdf1", "native-bdf2", "native-radau1", "native-radau3", "native-exprb2", "native-exprb43"}
H_DIVISORS = (1000, 2000, 4000, 8000)
THREAD_ENV_KEYS = ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "RAYON_NUM_THREADS", "JULIA_NUM_THREADS")


def applicability(case, spec):
    if "case_ids" in spec and case["case_id"] not in spec["case_ids"]:
        return "NOT_APPLICABLE:case_ids"
    if "models" in spec and case["model"] not in spec["models"]:
        return "NOT_APPLICABLE:models"
    if "max_dimension" in spec and len(case["initial_state"]) > spec["max_dimension"]:
        return "NOT_APPLICABLE:max_dimension"
    return None


def source_binding(root):
    root = Path(root)
    paths = [root / "Cargo.toml", root / "Cargo.lock", root / "tools" / "component_benchmark.py"]
    for optional in (root / "rust-toolchain.toml", root / ".cargo" / "config", root / ".cargo" / "config.toml"):
        if optional.exists(): paths.append(optional)
    paths.extend((root / "crates").rglob("*.rs"))
    paths.extend((root / "crates").rglob("Cargo.toml"))
    paths.extend((root / "crates").rglob("*.json"))
    paths.extend((root / "crates").rglob("*.csv"))
    if (root / "fixtures").exists():
        paths.extend(p for p in (root / "fixtures").rglob("*") if p.is_file())
    isolated_diffsol = root / "tools" / "benchmarks" / "diffsol" / "a07"
    if isolated_diffsol.exists():
        for pattern in ("*.rs", "Cargo.toml", "Cargo.lock", "*.json"):
            paths.extend(p for p in isolated_diffsol.rglob(pattern) if "target" not in p.parts)
    julia_project = root / "tools" / "benchmarks" / "component_julia"
    if julia_project.exists():
        paths.extend(p for p in julia_project.rglob("*") if p.is_file() and ".git" not in p.parts)
    paths.extend((root / "tools" / "component_benchmark").glob("*.py"))
    for name in ("cases.py", "models.py", "references.py", "scipy_adapter.py", "julia_adapter.jl", "diffsol_adapter.rs"):
        path = root / "tools" / "component_benchmark" / name
        if path.exists(): paths.append(path)
    prereg = root / "research" / "component_benchmark_20261018" / "PREREGISTRATION.md"
    if prereg.exists(): paths.append(prereg)
    return {str(path.relative_to(root)): file_digest(path) for path in sorted(set(paths))}


def executable_binding(config):
    import shutil
    bound = {}
    for solver_id, spec in config["solvers"].items():
        command = spec["command"]
        exe_path = Path(spec.get("cwd") or ".") / command[0]
        resolved = shutil.which(command[0]) or (str(exe_path.resolve()) if exe_path.exists() else None)
        if resolved is None:
            bound[solver_id] = {"status": "ENVIRONMENT_FAILURE", "executable": command[0]}
            continue
        paths = [Path(resolved)]
        for arg in command[1:]:
            candidate = Path(spec.get("cwd") or ".") / arg
            if candidate.is_file(): paths.append(candidate.resolve())
        for extra in spec.get("bind_files", []):
            candidate = Path(extra)
            if not candidate.is_file():
                raise FileNotFoundError("configured bind_file missing: " + extra)
            paths.append(candidate.resolve())
        for key in ("manifest", "executable_artifact"):
            if key in spec:
                candidate = Path(spec[key])
                if not candidate.is_file():
                    raise FileNotFoundError("configured " + key + " missing: " + str(candidate))
                paths.append(candidate.resolve())
        bound[solver_id] = {"status": "BOUND", "files": {str(p): file_digest(p) for p in paths}}
    return bound


def environment_binding(config):
    result = {}
    for solver_id, spec in config["solvers"].items():
        merged = dict(os.environ)
        merged.update(spec.get("env", {}))
        result[solver_id] = {key: merged.get(key) for key in THREAD_ENV_KEYS}
    return result


def load_config(path):
    data = strict_load(Path(path).read_text())
    if not isinstance(data, dict) or not isinstance(data.get("solvers"), dict):
        raise ValueError("config requires solvers object")
    for solver_id, spec in data["solvers"].items():
        if not isinstance(spec, dict) or not isinstance(spec.get("command"), list) or not spec["command"] or any(not isinstance(x, str) for x in spec["command"]):
            raise ValueError("invalid command for " + solver_id)
        if not isinstance(spec.get("equation_forms"), list) or not spec["equation_forms"]:
            raise ValueError("equation_forms required for " + solver_id)
        if "env" in spec and (not isinstance(spec["env"], dict) or any(not isinstance(k,str) or not isinstance(v,str) for k,v in spec["env"].items())):
            raise ValueError("invalid env for " + solver_id)
        for selector in ("case_ids", "models"):
            if selector in spec and (not isinstance(spec[selector], list) or any(not isinstance(x,str) for x in spec[selector])):
                raise ValueError("invalid " + selector + " for " + solver_id)
        if "max_dimension" in spec and (type(spec["max_dimension"]) is not int or spec["max_dimension"] < 1):
            raise ValueError("invalid max_dimension for " + solver_id)
    return data


def make_plan(cases, config, source, phase="P1", kind="screening"):
    if kind not in ("screening", "confirmation", "reference"):
        raise ValueError("invalid job kind")
    if len({c.get("case_id") for c in cases}) != len(cases):
        raise ValueError("duplicate case_id")
    if len(config.get("contrasts", [])) > 16:
        raise ValueError("at most sixteen frozen primary contrasts")
    if kind == "confirmation":
        global_contrasts = config.get("global_contrasts")
        if not isinstance(global_contrasts,list) or not 1 <= len(global_contrasts) <= 16 or len(config.get("contrasts",[])) != 1 or len(cases) != 1 or len(config["solvers"]) != 2:
            raise ValueError("confirmation requires one case/two solvers/one local contrast and frozen global contrasts")
        if canonical(config["contrasts"][0]) not in {canonical(x) for x in global_contrasts}:
            raise ValueError("local contrast absent from frozen global contrasts")
        local = config["contrasts"][0]
        if set(config["solvers"]) != {local["baseline"],local["candidate"]} or cases[0]["case_id"] != local["case_id"]:
            raise ValueError("confirmation case/solver Cartesian expansion forbidden")
    for contrast in config.get("contrasts", []):
        if contrast.get("case_id") not in {c["case_id"] for c in cases} or contrast.get("baseline") not in config["solvers"] or contrast.get("candidate") not in config["solvers"] or contrast.get("baseline") == contrast.get("candidate"):
            raise ValueError("invalid frozen contrast")
        for arm in ("baseline", "candidate"):
            config_id = contrast.get(arm + "_config_id", "default")
            if contrast[arm] in FIXED_SOLVERS and config_id not in {"h_divisor_" + str(x) for x in H_DIVISORS}:
                raise ValueError("fixed contrast requires explicit ladder configuration")
            if contrast[arm] not in FIXED_SOLVERS and config_id != "default":
                raise ValueError("nonfixed contrast configuration unknown")
    for case in cases:
        errors = validate_case(case)
        if errors: raise ValueError(f"{case.get('case_id')}: {errors}")
    case_gates = config.get("case_gates", {})
    if not isinstance(case_gates, dict):
        raise ValueError("case_gates must map case IDs to solver gates")
    for case_id, solver_gates in case_gates.items():
        if case_id not in {case["case_id"] for case in cases} or not isinstance(solver_gates, dict):
            raise ValueError("invalid case_gates case ID")
        for solver_id, gate in solver_gates.items():
            if solver_id not in config["solvers"] or not isinstance(gate, dict) or set(gate) != {"status", "reason", "evidence_sha256"}:
                raise ValueError("invalid case_gates solver gate")
            if gate["status"] not in ("PASS", "HOLD") or not isinstance(gate["reason"], str) or not gate["reason"] or not isinstance(gate["evidence_sha256"], str) or len(gate["evidence_sha256"]) != 64 or any(c not in "0123456789abcdef" for c in gate["evidence_sha256"]):
                raise ValueError("invalid case_gates status/reason/evidence_sha256")
    jobs = []
    allocation_jobs = []
    rng = random.Random(config.get("random_seed", 20261018))
    for case in cases:
        for round_index in range(17 if kind == "confirmation" else 1):
            solver_items = list(sorted(config["solvers"].items()))
            if kind == "confirmation": rng.shuffle(solver_items)
            for arm_order, (solver_id, spec) in enumerate(solver_items):
                divisors = spec.get("fixed_h_divisors", spec.get("h_divisors", H_DIVISORS if solver_id in FIXED_SOLVERS else (None,)))
                if solver_id in FIXED_SOLVERS and (tuple(divisors) != H_DIVISORS):
                    raise ValueError("fixed solver requires frozen h/h2/h4/h8 ladder")
                if kind == "confirmation" and solver_id in FIXED_SOLVERS:
                    arm = "baseline" if solver_id == config["contrasts"][0]["baseline"] else "candidate"
                    divisors = (int(config["contrasts"][0][arm + "_config_id"].removeprefix("h_divisor_")),)
                for divisor in divisors:
                    config_id = "h_divisor_" + str(divisor) if divisor is not None else "default"
                    request = {"case": case, "solver_id": solver_id}
                    if divisor is not None: request["solver_config"] = {"h_divisor": divisor}
                    excluded = applicability(case, spec)
                    capable = (case["equation_form"] in spec["equation_forms"] or (case["mass_matrix"] is None and "ODE" in spec["equation_forms"]) or (case["mass_matrix"] is not None and any(x in spec["equation_forms"] for x in ("DAE", "MASS_MATRIX")))) and case["reference_policy"] != "capability_gap"
                    prerequisite = case_gates.get(case["case_id"], {}).get(solver_id)
                    capability = excluded or ("SUPPORTED" if capable else "UNSUPPORTED_CAPABILITY")
                    if capability == "SUPPORTED" and prerequisite is not None and prerequisite["status"] == "HOLD":
                        capability = "HOLD_PREREQUISITE:" + prerequisite["reason"]
                    modes = ("timing",) if kind == "confirmation" else tuple(spec.get("measurement_modes", ("reference",) if kind == "reference" else (("timing", "allocation") if spec.get("allocation_mode", solver_id.startswith(("native-", "diffsol-"))) and round_index == 0 else ("timing",))))
                    if any(mode not in ({"reference"} if kind == "reference" else {"timing", "allocation"}) for mode in modes):
                        raise ValueError("measurement mode incompatible with job kind")
                    for mode in modes:
                        identity = {"request": request, "solver_spec": spec, "kind": kind, "round": round_index, "mode": mode, "source": source}
                        if prerequisite is not None: identity["prerequisite"] = prerequisite
                        target_jobs = allocation_jobs if mode == "allocation" else jobs
                        job = {"job_id": digest(identity), "case_id": case["case_id"], "solver_id": solver_id,
                                     "config_id": config_id, "round": round_index, "arm_order": arm_order, "kind": kind,
                                     "measurement_mode": mode, "request": request,
                                     "capability": capability}
                        if prerequisite is not None: job["prerequisite"] = prerequisite
                        target_jobs.append(job)
    jobs.extend(allocation_jobs)
    return {"schema": SCHEMA, "phase": phase, "kind": kind, "source_binding": source,
            "config_sha256": digest(config), "solvers": config["solvers"], "executable_binding": executable_binding(config),
            "environment_binding": environment_binding(config), "jobs": jobs,
            "global_multiplicity": len(config["global_contrasts"]) if kind == "confirmation" else len(config.get("contrasts",[])),
            "global_contrasts_sha256": digest(config["global_contrasts"]) if kind == "confirmation" else None,
            "timing_authority": "diagnostic_only", "production_admission": "HOLD"}


def atomic_json(path, value):
    path = Path(path)
    temp = path.with_name(path.name + ".tmp-" + str(os.getpid()))
    temp.write_text(json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n")
    os.replace(temp, path)


def save_receipt(folder, value):
    atomic_json(folder / "receipt.json", value)
    temp = folder / ("receipt.jsonl.tmp-" + str(os.getpid()))
    temp.write_text(canonical(value) + "\n")
    os.replace(temp, folder / "receipt.jsonl")


def validate_receipt(folder, receipt, job, source_hash):
    folder = Path(folder)
    if receipt.get("job") != job or receipt.get("source_binding_sha256") != source_hash:
        return False
    if not (folder / "request.json").exists() or file_digest(folder / "request.json") != receipt.get("request_sha256"):
        return False
    if receipt.get("result_sha256") != (digest(receipt["result"]) if receipt.get("result") is not None else None):
        return False
    if not (folder / "receipt.jsonl").exists() or (folder / "receipt.jsonl").read_text() != canonical(receipt) + "\n":
        return False
    for attempt in receipt.get("attempts", []):
        for key in ("stdout", "stderr"):
            path = attempt.get(key + "_path")
            if path and (not Path(path).is_file() or file_digest(path) != attempt.get(key + "_sha256")):
                return False
        report = attempt.get("gnu_time_report")
        if report and (not Path(report).is_file() or file_digest(report) != attempt.get("gnu_time_report_sha256")):
            return False
    return True


def _run_one(job, solver, output, timeout, source_digest, hard_stop_monotonic, retry=0, diagnosis=None,
             allocation=None, tracker=None):
    output = Path(output)
    folder = output / "jobs" / job["job_id"]
    folder.mkdir(parents=True, exist_ok=True)
    atomic_json(folder / "request.json", job["request"])
    prior = folder / "receipt.json"
    attempts = strict_load(prior.read_text()).get("attempts", []) if prior.exists() else []
    if job["capability"] != "SUPPORTED":
        status = ("HOLD_PREREQUISITE" if job["capability"].startswith("HOLD_PREREQUISITE:") else
                  "NOT_APPLICABLE" if job["capability"].startswith("NOT_APPLICABLE:") else "UNSUPPORTED_CAPABILITY")
        result = {"status": status, "reason": job["capability"], "times": [], "states": [], "counters": None, "timing": None, "allocations": None}
        final = {"schema": SCHEMA, "job": job, "result": result, "attempts": attempts, "source_binding_sha256": source_digest,
                 "request_sha256": file_digest(folder / "request.json"), "result_sha256": digest(result), "status": status}
        if "prerequisite" in job: final["prerequisite"] = job["prerequisite"]
        save_receipt(folder, final)
        return final
    argv = solver["command"] + ["--request", str((folder / "request.json").resolve())]
    if job["measurement_mode"] == "allocation": argv += ["--measurement-mode", "allocation"]
    env = dict(os.environ)
    env.update(solver.get("env", {}))
    if allocation is not None:
        env = hpc_resources.thread_environment(env, allocation)
    observed_env = {k:env[k] for k in THREAD_ENV_KEYS if k in env}
    observed_env.update(solver.get("env", {}))
    if retry not in (0,1) or (retry == 1 and not diagnosis):
        raise ValueError("retry requires an explicit infrastructure diagnosis")
    if len(attempts) > 1 or (attempts and not diagnosis):
        raise ValueError("retry exhausted or diagnosis missing")
    attempt_indices = [1] if attempts else list(range(retry + 1))
    for attempt_index in attempt_indices:
        remaining = hard_stop_monotonic - time.monotonic()
        if remaining <= 0:
            attempts.append({"raw_status": "BUDGET_TIMEOUT_HOLD", "reason": "host_lease_deadline", "retry_diagnosis": diagnosis if attempt_index else None})
            break
        begin = time.time()
        raw_status = "ENVIRONMENT_FAILURE"
        parsed = None
        proc = None
        try:
            if allocation is None:
                proc = subprocess.run(argv, cwd=solver.get("cwd"), env=env, capture_output=True, text=True, timeout=min(timeout, remaining), check=False)
                stdout, stderr, exit_code = proc.stdout, proc.stderr, proc.returncode
                launch_argv = argv
            else:
                proc = hpc_resources.run_process(argv, allocation=allocation, tracker=tracker, env=env,
                                                 cwd=solver.get("cwd"), timeout=min(timeout, remaining),
                                                 metrics_path=folder / f"attempt-{attempt_index}.time")
                stdout, stderr, exit_code = proc["stdout"].decode(errors="replace"), proc["stderr"].decode(errors="replace"), proc["exit_code"]
                launch_argv = proc["command"]
                if proc["timed_out"]:
                    raise subprocess.TimeoutExpired(launch_argv, min(timeout, remaining), output=proc["stdout"], stderr=proc["stderr"])
            try:
                parsed = strict_load(stdout)
                errs = validate_result(parsed, job["request"]["case"], job["measurement_mode"])
                raw_status = parsed.get("status") if not errs else "PROTOCOL_FAILURE"
                if exit_code != 0 and raw_status == "OK":
                    raw_status, errs = "PROTOCOL_FAILURE", ["ok_with_nonzero_exit"]
            except (ValueError, TypeError) as exc:
                errs = [str(exc)]
                raw_status = "ENVIRONMENT_FAILURE" if exit_code != 0 else "PROTOCOL_FAILURE"
        except subprocess.TimeoutExpired as exc:
            stdout = exc.stdout.decode(errors="replace") if isinstance(exc.stdout, bytes) else (exc.stdout or "")
            stderr = exc.stderr.decode(errors="replace") if isinstance(exc.stderr, bytes) else (exc.stderr or "")
            exit_code, errs, raw_status = None, ["timeout"], "ENVIRONMENT_FAILURE"
            launch_argv = hpc_resources.affinity_command(argv, allocation) if allocation is not None else argv
        except (OSError, RuntimeError, ValueError) as exc:
            stdout, stderr, exit_code, errs = "", str(exc), None, ["spawn_error"]
            launch_argv = argv
        prefix = f"attempt-{attempt_index}"
        (folder / (prefix + ".stdout")).write_text(stdout)
        (folder / (prefix + ".stderr")).write_text(stderr)
        attempt = {"argv": argv, "started_unix": begin, "ended_unix": time.time(), "exit_code": exit_code,
                   "raw_status": raw_status, "protocol_errors": errs, "stdout_path": str(folder / (prefix + ".stdout")),
                   "stderr_path": str(folder / (prefix + ".stderr")), "stdout_sha256": file_digest(folder / (prefix + ".stdout")),
                   "stderr_sha256": file_digest(folder / (prefix + ".stderr")), "timeout_seconds": min(timeout, remaining),
                   "retry_diagnosis": diagnosis if attempt_index else None, "observed_thread_environment": observed_env}
        if allocation is not None:
            attempt["launch_argv"] = launch_argv
            attempt["affinity_cpus"] = list(allocation.cpus)
            attempt["reserved_memory_bytes"] = allocation.memory_bytes
            if isinstance(proc, dict):
                for key in ("peak_rss_bytes", "peak_live_rss_bytes", "rss_samples", "rss_source",
                            "affinity_escape", "affinity_verified", "cgroup_oom_events_delta", "cgroup_oom_scope",
                            "assigned_cpu_busy_fraction", "assigned_cpu_steal_fraction",
                            "assigned_cpu_sample_source", "gnu_time_max_rss_bytes", "gnu_time_report"):
                    attempt[key] = proc[key]
                metrics = folder / f"attempt-{attempt_index}.time"
                if metrics.is_file():
                    attempt["gnu_time_report_sha256"] = file_digest(metrics)
        attempts.append(attempt)
        # Only infrastructure faults get the single diagnosed retry.
        if raw_status != "ENVIRONMENT_FAILURE": break
    status = attempts[-1]["raw_status"]
    final = {"schema": SCHEMA, "job": job, "result": parsed if status in ("OK", "UNSUPPORTED_CAPABILITY", "NUMERICAL_FAILURE") else None,
             "attempts": attempts, "source_binding_sha256": source_digest, "status": status,
             "request_sha256": file_digest(folder / "request.json"),
             "result_sha256": digest(parsed) if parsed is not None and status in ("OK", "UNSUPPORTED_CAPABILITY", "NUMERICAL_FAILURE") else None,
             "timing_authority": "diagnostic_only"}
    if "prerequisite" in job: final["prerequisite"] = job["prerequisite"]
    save_receipt(folder, final)
    return final


def run_plan(plan, output, root, ledger_path, concurrency=1, resume=False, cleanup_seconds=30, lease_id=None, retry_diagnosis=None, max_jobs=None, hpc=False):
    if type(concurrency) is not int or concurrency < 1:
        raise ValueError("concurrency must be positive")
    if plan["kind"] == "confirmation" and concurrency != 1:
        raise ValueError("confirmation requires serial randomized arm order")
    if hpc and plan["kind"] == "confirmation":
        raise RuntimeError("HPC_CONFIRMATION_HOLD")
    if hpc and lease_id is None:
        raise ValueError("HPC mode requires an existing persistent host lease")
    if ledger_path is None:
        raise ValueError("shared campaign ledger path required")
    if retry_diagnosis and not resume:
        raise ValueError("diagnosed retry requires resume")
    if max_jobs is not None and (type(max_jobs) is not int or max_jobs < 1):
        raise ValueError("max_jobs must be positive")
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    if source_binding(root) != plan["source_binding"]:
        raise RuntimeError("SOURCE_BINDING_MISMATCH")
    if executable_binding({"solvers": plan["solvers"]}) != plan["executable_binding"]:
        raise RuntimeError("EXECUTABLE_BINDING_MISMATCH")
    if environment_binding({"solvers": plan["solvers"]}) != plan["environment_binding"]:
        raise RuntimeError("THREAD_ENVIRONMENT_BINDING_MISMATCH")
    plan_path = output / "plan.json"
    if plan_path.exists() and strict_load(plan_path.read_text()) != plan:
        raise RuntimeError("PLAN_IDENTITY_MISMATCH")
    if not plan_path.exists(): atomic_json(plan_path, plan)
    pending = []
    retry_ids = set()
    for job in plan["jobs"]:
        receipt = output / "jobs" / job["job_id"] / "receipt.json"
        if receipt.exists():
            old = strict_load(receipt.read_text())
            if not validate_receipt(receipt.parent, old, job, digest(plan["source_binding"])):
                raise RuntimeError("RECEIPT_IDENTITY_MISMATCH")
            if not resume: raise RuntimeError("EXISTING_RECEIPT_USE_RESUME")
            if retry_diagnosis and old["status"] == "ENVIRONMENT_FAILURE" and len(old.get("attempts", [])) == 1:
                pending.append(job)
                retry_ids.add(job["job_id"])
            continue
        pending.append(job)
    if not pending: return []
    timeout = {"screening": 60, "confirmation": 600, "reference": 1200}[plan["kind"]]
    source_hash = digest(plan["source_binding"])
    synthetic = [_run_one(j, plan["solvers"][j["solver_id"]], output, timeout, source_hash, float("inf")) for j in pending if j["capability"] != "SUPPORTED"]
    pending = [j for j in pending if j["capability"] == "SUPPORTED"]
    if max_jobs is not None: pending = pending[:max_jobs]
    if not pending: return synthetic
    # Conservative host wall reservation includes both attempts and cleanup.
    pool = None
    if hpc:
        resources = hpc_resources.detect_resources()
        cap = None if concurrency == 1 else concurrency
        pool = hpc_resources.ResourcePool(resources, max_workers=cap, telemetry_path=output / "hpc_telemetry.jsonl")
        demand = lambda job: hpc_resources.demand_for(job, plan["solvers"][job["solver_id"]], plan["kind"])
        reserved_seconds = hpc_resources.estimate_waves(pending, resources, demand=demand,
                                                        max_workers=pool.max_workers) * timeout
    else:
        reserved_seconds = ((len(pending) + concurrency - 1) // concurrency) * timeout
    lease = lease_id or reserve(ledger_path, plan["phase"], reserved_seconds, platform.node(), cleanup_seconds)
    with locked_ledger(ledger_path) as ledger:
        record = next(x for x in ledger["leases"] if x["id"] == lease)
        if record["host"] != platform.node() or record["phase"] != plan["phase"] or "end" in record:
            raise RuntimeError("LEASE_IDENTITY_MISMATCH")
        if lease_id and record["deadline"] - time.time() < reserved_seconds + cleanup_seconds:
            raise RuntimeError("LEASE_REMAINING_INSUFFICIENT")
        hard_stop = record["start_monotonic"] + record["reserved_seconds"] - cleanup_seconds
    results = list(synthetic)
    try:
        if hpc:
            results.extend(pool.run(pending,
                lambda job, allocation, tracker: _run_one(job, plan["solvers"][job["solver_id"]], output,
                    timeout, source_hash, hard_stop, 1 if job["job_id"] in retry_ids else 0,
                    retry_diagnosis if job["job_id"] in retry_ids else None, allocation, tracker),
                demand=demand))
        else:
            with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
                futures = [executor.submit(_run_one, job, plan["solvers"][job["solver_id"]], output, timeout, source_hash, hard_stop,
                                       1 if job["job_id"] in retry_ids else 0,
                                       retry_diagnosis if job["job_id"] in retry_ids else None) for job in pending]
                for future in concurrent.futures.as_completed(futures):
                    results.append(future.result())
    finally:
        if lease_id is None:
            close(ledger_path, lease)
    if source_binding(root) != plan["source_binding"]:
        atomic_json(output / "source_drift.json", {"status": "SOURCE_BINDING_MISMATCH", "before": plan["source_binding"], "after": source_binding(root)})
        raise RuntimeError("SOURCE_BINDING_MISMATCH_AFTER_RUN")
    return results
