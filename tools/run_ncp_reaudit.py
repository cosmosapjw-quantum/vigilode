#!/usr/bin/env python3
"""Source-bound, resumable NCP execution; never installs or promotes results.

Run from the published, clean checkout. Output and Cargo target directories must
be outside it. Commands execute sequentially. The promotion gate, not this
runner, decides whether raw measurements support a scientific claim.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import math
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import time

from component_benchmark import hpc

SCHEMA = "vigilode.ncp-receipt.v1"
POLICY = {"version": 1, "files": "all git tracked files; symlinks bind link text, resolved in-repo regular-file path and target bytes; external/broken/cyclic targets rejected",
          "dirty": "reject tracked changes and nonignored untracked files",
          "output": "outside repository", "dependencies": "Cargo locked offline; exact lock hashes",
          "resume": "same source, config, tool versions, relevant environment, host boot, CPU allocation"}
ENV_KEYS = ("PATH", "LD_LIBRARY_PATH", "LD_PRELOAD", "LD_AUDIT", "LD_BIND_NOW", "GLIBC_TUNABLES",
            "MALLOC_ARENA_MAX", "MALLOC_CONF", "OMP_PROC_BIND", "OMP_PLACES", "OMP_DYNAMIC", "OMP_WAIT_POLICY",
            "KMP_AFFINITY", "GOMP_CPU_AFFINITY", "MKL_DYNAMIC", "OPENBLAS_CORETYPE",
            "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_HOME",
            "RUSTC", "CC", "CXX", "FC", "AR", "PKG_CONFIG_PATH", "LIBRARY_PATH",
            "CPATH", "C_INCLUDE_PATH", "CPLUS_INCLUDE_PATH", "PYTHONPATH")
LIBS = ("sundials_cvode", "sundials_nvecserial", "sundials_sunmatrixdense",
        "sundials_sunmatrixband", "sundials_sunlinsoldense", "sundials_sunlinsolband", "blas", "gfortran", "m")
BOUND_CONTRACTS = {
    "diagonal": (1e-7, "REQUESTED_ODE_ENDPOINTS_ONLY", "PASS_CERTIFIED_ENDPOINTS", False),
    "global": (1e-4, "BOUND_MODEL_FULL_CURVE", "PASS_CERTIFIED_FULL_CURVE", True),
    "allocation": (0.5, "ALGEBRAIC_ALLOCATION_ONLY", "PASS_VALIDATED_ALLOCATION", False),
}
EXPECTED_CAMPAIGN = {
    **{("polynomial", case): ["protected", "krylov", "leja", "laguerre", "chebyshev", "router"]
       for case in ("p4", "p16", "p64")},
    **{("homotopy", case): ["direct", "continuation"] for case in ("h3", "h8", "h16")},
    **{("actual-k", case): ["direct", "continuation"] for case in ("k2", "k4")},
    ("bound-clients", "diagonal"): ["bounded", "resolvent", "diagonal"],
    ("bound-clients", "global"): ["provider", "replay"],
    ("bound-clients", "allocation"): ["uniform", "optimized"],
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()


def read_json(path):
    def invalid(value):
        raise ValueError(f"non-finite JSON token: {value}")
    return json.loads(Path(path).read_text(), parse_constant=invalid)


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n")
    tmp.replace(path)


def capture(argv, cwd=None, timeout=30, **kwargs):
    try:
        p = subprocess.run(argv, cwd=cwd, capture_output=True, timeout=timeout, **kwargs)
        return {"command": argv, "exit_code": p.returncode,
                "stdout": p.stdout.decode(errors="replace"), "stderr": p.stderr.decode(errors="replace")}
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {"command": argv, "exit_code": None, "stdout": "", "stderr": str(exc)}


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


def source_identity(repo):
    records = []
    for raw in git(repo, "ls-files", "-z").split(b"\0"):
        if not raw:
            continue
        rel = os.fsdecode(raw)
        path = repo / rel
        resolved = path.resolve(strict=True)
        if repo not in resolved.parents or not resolved.is_file():
            raise ValueError("tracked path resolves outside in-repo regular-file closure: " + rel)
        if path.is_symlink():
            records.append([rel, "symlink", os.readlink(path), str(resolved.relative_to(repo)), digest(resolved.read_bytes())])
        else:
            records.append([rel, "file", str(resolved.relative_to(repo)), digest(resolved.read_bytes())])
    dirty = git(repo, "status", "--porcelain=v1", "--untracked-files=normal").decode(errors="replace")
    return {"commit": git(repo, "rev-parse", "HEAD").decode().strip(),
            "tree": git(repo, "rev-parse", "HEAD^{tree}").decode().strip(),
            "closure_sha256": digest(canonical(records)),
            "closure_policy_sha256": digest(canonical(POLICY)),
            "tracked_files": len(records), "clean": not dirty, "dirty_status": dirty}


def outside_output(repo, output):
    repo, output = repo.resolve(), output.resolve()
    if output == repo or repo in output.parents:
        raise ValueError("output must be outside the repository")
    return output


def choose_cpus(spec, maximum):
    allowed = tuple(sorted(os.sched_getaffinity(0)))
    selected = allowed[:maximum] if spec == "auto" else hpc.parse_cpu_list(spec)
    if len(selected) < maximum or not set(selected).issubset(allowed):
        raise ValueError(f"need at least {maximum} allowed logical CPUs; allowed={allowed}, selected={selected}")
    return selected


def host_identity(cpus):
    def read(path):
        try:
            return Path(path).read_text().strip()
        except OSError:
            return None
    model = sorted({line for line in (read("/proc/cpuinfo") or "").splitlines()
                    if line.startswith(("model name", "vendor_id", "flags"))})
    return {"hostname": platform.node(), "boot_id": read("/proc/sys/kernel/random/boot_id"),
            "kernel": platform.release(), "machine": platform.machine(), "cpu_model": model,
            "allowed_cpus": sorted(os.sched_getaffinity(0)), "selected_cpus": list(cpus),
            "cpu_quota_cores": hpc._quota_cores(),
            "siblings": {str(cpu): read(f"/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list") for cpu in cpus},
            "isolation": "logical CPU affinity only; no physical-core exclusivity or uncontended-host claim",
            "provider": "not inferred; this program does not connect to or provision NCP"}


def validate_config(config, repo):
    if config.get("schema") != "vigilode.ncp-config.v1":
        raise ValueError("unsupported config schema")
    if config["warmups"] != 2 or config["repetitions"] != 21:
        raise ValueError("frozen protocol requires 2 warmups and 21 repetitions")
    if config["threads"] != [1, 2, 4, 8]:
        raise ValueError("frozen thread matrix must be [1,2,4,8]")
    for key in ("memory_limit_bytes", "validation_timeout_seconds", "comparison_timeout_seconds",
                "work_precision_timeout_seconds", "invocation_timeout_seconds"):
        if type(config[key]) is not int or config[key] <= 0:
            raise ValueError("invalid positive resource budget: " + key)
    seen = set()
    for case in config["comparison_cases"]:
        names = [case["suite"], case["case"], *case["arms"]]
        if any(not isinstance(name, str) or not re.fullmatch(r"[a-z0-9][a-z0-9-]*", name) for name in names):
            raise ValueError("unsafe case/arm identifier")
        key = (case["suite"], case["case"])
        if key in seen or not case["arms"] or len(set(case["arms"])) != len(case["arms"]):
            raise ValueError("duplicate/empty case or arm")
        if EXPECTED_CAMPAIGN.get(key) != case["arms"]:
            raise ValueError("frozen case/arm matrix changed")
        seen.add(key)
        expected_budget = (BOUND_CONTRACTS.get(case["case"], (None,))[0] if case["suite"] == "bound-clients"
                           else {"polynomial": 1e-7, "homotopy": 2e-5, "actual-k": 1e-9}.get(case["suite"]))
        if expected_budget is None or case["original_budget"] != expected_budget:
            raise ValueError("changed frozen original budget")
    if seen != set(EXPECTED_CAMPAIGN):
        raise ValueError("incomplete frozen comparison campaign")
    for rel, expected in config["locked_inputs"].items():
        if digest((repo / rel).read_bytes()) != expected:
            raise ValueError(f"locked input changed: {rel}")
    corpus = read_json(repo / config["corpus"])
    if (corpus["warmups"], corpus["repetitions"]) != (2, 21):
        raise ValueError("corpus repetition protocol changed")
    if (config["corpus"] != "crates/vigilode-bench/protocols/n02_independent_reference_v1.json"
            or corpus.get("reference", {}).get("independent_protocol", {}).get("tier") != "radau5_cvode_refinement_v1"):
        raise ValueError("strict N02 independent reference protocol required")


def environment_identity():
    env = {key: os.environ.get(key) for key in ENV_KEYS}
    cargo_home = Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo")))
    env["cargo_configuration"] = {
        str(path): digest(path.read_bytes()) for path in [cargo_home / "config", cargo_home / "config.toml"] if path.is_file()}
    return env


def preflight(repo, output, config, source, dry_run):
    issues, tools = [], {}
    if not source["clean"]:
        issues.append("SOURCE_NOT_CLEAN")
    for name in ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET"):
        if os.environ.get(name):
            issues.append(f"UNSUPPORTED_ENVIRONMENT:{name}")
    for name in ("rustc", "cargo", "gcc", "gfortran", "ar", "nm", "objcopy", "sha256sum", "taskset", "ldd"):
        path = shutil.which(name)
        result = capture([path, "--version"]) if path else {"exit_code": None, "stderr": "not found", "stdout": ""}
        tools[name] = {"path": path, **result}
        if result["exit_code"] != 0:
            issues.append(f"TOOL_UNAVAILABLE:{name}")
    for name in ("rustc", "cargo"):
        words = tools[name]["stdout"].split()
        if len(words) < 2 or words[1] != config["rust_version"]:
            issues.append(f"TOOLCHAIN_VERSION:{name}")
    gnu_time = Path("/usr/bin/time")
    tools["gnu_time"] = capture([str(gnu_time), "--version"]) if gnu_time.is_file() else {"exit_code": None}
    if tools["gnu_time"]["exit_code"] != 0:
        issues.append("GNU_TIME_REQUIRED_FOR_SHORT_PROCESS_RSS")
    packages = {}
    for package in ("pytest", "numpy", "scipy", "mpmath"):
        try:
            packages[package] = importlib.metadata.version(package)
        except importlib.metadata.PackageNotFoundError:
            packages[package] = None
            issues.append(f"PYTHON_PACKAGE_MISSING:{package}")
    probe = {"status": "PLANNED"}
    resources = {}
    try:
        available = hpc._mem_available()
        remaining, maximum, current = hpc._cgroup_memory()
        quota = hpc._quota_cores()
        resources = {"host_mem_available_bytes": available, "cgroup_remaining_bytes": remaining,
                     "cgroup_max_bytes": maximum, "cgroup_current_bytes": current, "cpu_quota_cores": quota,
                     "scope": "admission observation only; no exclusive host reservation or enforced process RSS ceiling"}
        if min(available, remaining if remaining is not None else available) < config["memory_limit_bytes"]:
            issues.append("MEMORY_ADMISSION_INSUFFICIENT")
        if quota is not None and quota < max(config["threads"]):
            issues.append("CPU_QUOTA_BELOW_THREAD_MATRIX")
    except (OSError, RuntimeError, ValueError) as exc:
        issues.append("RESOURCE_ADMISSION_UNKNOWN:" + str(exc))
    if not dry_run and tools["gcc"]["exit_code"] == 0:
        source_code = ("#include <sundials/sundials_context.h>\n#include <sundials/sundials_config.h>\n"
                       "#include <cvode/cvode.h>\n#include <stdio.h>\n"
                       "int main(void){SUNContext c=NULL; if(SUNContext_Create(NULL,&c))return 1;"
                       "void *m=CVodeCreate(CV_BDF,c);if(!m)return 2;CVodeFree(&m);SUNContext_Free(&c);"
                       "puts(SUNDIALS_VERSION);return 0;}\n")
        binary = output / "preflight" / "native_probe"
        binary.parent.mkdir(parents=True, exist_ok=True)
        probe = capture(["gcc", "-x", "c", "-", "-o", str(binary), *["-l" + lib for lib in LIBS]],
                        input=source_code.encode())
        if probe["exit_code"] == 0:
            probe["runtime"] = capture([str(binary)])
            try:
                probe["runtime_libraries"] = runtime_dependencies(binary)
            except (OSError, ValueError) as exc:
                issues.append("NATIVE_LIBRARY_IDENTITY_UNAVAILABLE:" + str(exc))
        if probe["exit_code"] != 0 or probe.get("runtime", {}).get("exit_code") != 0:
            issues.append("NATIVE_HEADERS_OR_LINK_RUNTIME_UNAVAILABLE")
    elif not dry_run:
        issues.append("NATIVE_PROBE_NOT_EXECUTED")
    return {"schema": SCHEMA, "issues": issues, "tools": tools, "packages": packages,
            "python": sys.version, "native_probe": probe, "resources": resources,
            "status": "HOLD" if issues else ("PLANNED" if dry_run else "PASS")}


def steps(config):
    cargo = ["cargo"]
    locked = ["--locked", "--offline", "-j1"]
    bench = ["--manifest-path", "crates/vigilode-bench/Cargo.toml"]
    return [
        ("metadata_root", cargo + ["metadata", "--locked", "--offline", "--format-version", "1"], "native"),
        ("metadata_bench", cargo + ["metadata", *bench, "--locked", "--offline", "--format-version", "1"], "bench"),
        ("format", cargo + ["fmt", "--all", "--check"], "native"),
        ("bench_format", cargo + ["fmt", *bench, "--all", "--check"], "bench"),
        ("workspace_tests", cargo + ["test", "--workspace", "--lib", "--tests", *locked, "--", "--test-threads=1"], "native"),
        ("workspace_doctests", cargo + ["test", "--workspace", "--doc", *locked, "--", "--test-threads=1"], "native"),
        ("bench_tests", cargo + ["test", *bench, *locked, "--", "--test-threads=1"], "bench"),
        ("clippy", cargo + ["clippy", "-p", "vigilode", "-p", "rodas5p-core", "--all-targets", "--no-deps",
                             *locked, "--", "-D", "warnings"], "clippy"),
        ("python_contracts", [sys.executable, "-m", "pytest", "-q", "tools/tests"], "native"),
        ("comparison_build", cargo + ["build", "-p", "rodas5p-integrators", "--examples", "--release", *locked], "native"),
        ("bench_build", cargo + ["build", *bench, "--release", *locked], "bench"),
    ]


def planned_case_ids(config):
    ids = []
    for case in config["comparison_cases"]:
        for threads in config["threads"]:
            for repetition in range(-config["warmups"], config["repetitions"]):
                phase = f"warmup{repetition + config['warmups']:02}" if repetition < 0 else f"repeat{repetition:02}"
                offset = (repetition + config["warmups"]) % len(case["arms"])
                for arm in case["arms"][offset:] + case["arms"][:offset]:
                    ids.append(f"{case['suite']}-{case['case']}-t{threads}-{phase}-{arm}")
    return ids + ["work_precision"]


def verify_files(receipt, output):
    required = {f"runs/{receipt['id']}/stdout.log", f"runs/{receipt['id']}/stderr.log"}
    if not required.issubset(receipt.get("artifacts", {})):
        raise ValueError("resume receipt lacks required artifacts")
    for rel, expected in receipt.get("artifacts", {}).items():
        path = (output / rel).resolve()
        if output.resolve() not in path.parents or not path.is_file() or digest(path.read_bytes()) != expected:
            raise ValueError(f"resume artifact missing or modified: {rel}")


def produced_binaries(identifier, output):
    if identifier == "comparison_build":
        return [output / "targets/native/release/examples" / name
                for name in ("reaudit_comparison", "reaudit_bound_clients")]
    if identifier == "bench_build":
        return [output / "targets/bench/release/vigilode-bench"]
    return []


def runtime_dependencies(binary):
    result = capture(["ldd", str(binary)])
    if result["exit_code"] != 0 or "not found" in result["stdout"]:
        raise ValueError("dynamic-library resolution failed: " + str(binary))
    libraries = {}
    for line in result["stdout"].splitlines():
        match = re.match(r"\s*(?:\S+\s+=>\s+)?(/.+?)\s+\(", line)
        if match:
            resolved = Path(match.group(1)).resolve(strict=True)
            libraries[str(resolved)] = digest(resolved.read_bytes())
    if not libraries:
        raise ValueError("no resolved dynamic libraries; runtime closure unknown")
    return {"libraries": libraries, "scope": "ldd-resolved shared libraries including loader; no claim about arbitrary dlopen/data assets"}


def executable_binding(argv, output):
    executable = Path(argv[0])
    for build in ("comparison_build", "bench_build"):
        if executable in produced_binaries(build, output):
            receipt = read_json(output / "runs" / build / "RECEIPT.json")
            rel = str(executable.relative_to(output))
            expected = receipt.get("artifacts", {}).get(rel)
            if not expected or not executable.is_file() or digest(executable.read_bytes()) != expected:
                raise ValueError("benchmark executable differs from source-bound build: " + rel)
            runtime = runtime_dependencies(executable)
            if receipt.get("runtime_libraries", {}).get(rel) != runtime:
                raise ValueError("benchmark shared libraries changed or identity absent: " + rel)
            return {"path": rel, "sha256": expected, "runtime_libraries": runtime}
    return None


def execute(identifier, argv, repo, output, source, config, cpus, target, timeout, resume, raw_file=None):
    directory = output / "runs" / identifier
    receipt_path = directory / "RECEIPT.json"
    executable = executable_binding(argv, output)
    if receipt_path.is_file():
        if not resume:
            raise ValueError(f"receipt already exists: {identifier}; use --resume or a new output directory")
        receipt = read_json(receipt_path)
        if (receipt.get("source") != source or receipt.get("argv") != argv
                or receipt.get("execution_identity_sha256") != config["_execution_identity_sha256"]):
            raise ValueError(f"resume request changed: {identifier}")
        verify_files(receipt, output)
        for binary in produced_binaries(identifier, output):
            rel = str(binary.relative_to(output))
            if receipt.get("runtime_libraries", {}).get(rel) != runtime_dependencies(binary):
                raise ValueError("resume build shared libraries changed or identity absent")
        if receipt.get("executable_binding") != executable:
            raise ValueError("resume benchmark executable binding changed")
        return receipt
    directory.mkdir(parents=True, exist_ok=True)
    if raw_file is not None and raw_file.exists():
        raise ValueError("unreceipted raw output already exists; preserve it and use a fresh output directory")
    if any(path.exists() for path in produced_binaries(identifier, output)):
        raise ValueError("unreceipted release binary already exists; preserve it and use a fresh output directory")
    if source_identity(repo) != source:
        raise ValueError("source changed before command")
    allocation = hpc.Allocation(tuple(cpus), config["memory_limit_bytes"], "native")
    env = hpc.thread_environment(os.environ, allocation, contract=True)
    env.update({"CARGO_TARGET_DIR": str(output / "targets" / target), "CARGO_NET_OFFLINE": "true",
                "CARGO_BUILD_JOBS": "1", "PYTHONDONTWRITEBYTECODE": "1"})
    python_path = str(repo / "tools")
    env["PYTHONPATH"] = python_path + (os.pathsep + env["PYTHONPATH"] if env.get("PYTHONPATH") else "")
    try:
        result = hpc.run_process(argv, allocation=allocation, tracker=hpc.ProcessTracker(), env=env,
                                 cwd=repo, timeout=timeout, metrics_path=directory / "gnu_time.txt")
    except (OSError, RuntimeError, subprocess.SubprocessError) as exc:
        result = {"stdout": b"", "stderr": str(exc).encode(), "exit_code": None, "timed_out": False,
                  "elapsed_seconds": None, "peak_rss_bytes": None, "rss_status": "UNAVAILABLE",
                  "affinity_verified": False, "execution_error": type(exc).__name__}
    artifacts = {}
    for key in ("stdout", "stderr"):
        path = directory / (key + ".log")
        path.write_bytes(result.pop(key))
        artifacts[str(path.relative_to(output))] = digest(path.read_bytes())
    for path in [directory / "gnu_time.txt", raw_file, *produced_binaries(identifier, output)]:
        if path is not None and path.is_file():
            artifacts[str(path.relative_to(output))] = digest(path.read_bytes())
    after = source_identity(repo)
    binaries_present = all(path.is_file() for path in produced_binaries(identifier, output))
    runtime_libraries = {}
    runtime_bound = True
    for binary in produced_binaries(identifier, output):
        try:
            runtime_libraries[str(binary.relative_to(output))] = runtime_dependencies(binary)
        except (OSError, ValueError) as exc:
            runtime_bound = False
            result["runtime_identity_error"] = str(exc)
    try:
        binary_unchanged = executable_binding(argv, output) == executable
    except (OSError, ValueError):
        binary_unchanged = False
    okay = (result["exit_code"] == 0 and not result["timed_out"] and after == source
            and binaries_present and binary_unchanged and runtime_bound and (raw_file is None or raw_file.is_file()))
    receipt = {"schema": SCHEMA, "id": identifier, "argv": argv, "source": source,
               "execution_identity_sha256": config["_execution_identity_sha256"],
               "executable_binding": executable, "executable_unchanged_after": binary_unchanged,
               "runtime_libraries": runtime_libraries,
               "source_after": after, "status": "PASS" if okay else "HOLD", "process": result,
               "target_directory": env["CARGO_TARGET_DIR"], "thread_environment": {k: env[k] for k in hpc.THREAD_KEYS},
               "artifacts": artifacts}
    write_json(receipt_path, receipt)
    return receipt


def numeric_row_issues(row, case, arm, threads):
    issues = []
    stage = case["suite"] == "actual-k"
    expected_status = "PASS_CERTIFIED_STAGE_FUNCTIONAL" if stage else "PASS_CERTIFIED_ENDPOINTS"
    expected_scope = "STAGE_FUNCTIONAL_ONLY" if stage else "REQUESTED_ODE_ENDPOINTS_ONLY"
    full_curve = False
    bound_client = case["suite"] == "bound-clients"
    if bound_client:
        _, expected_scope, expected_status, full_curve = BOUND_CONTRACTS[case["case"]]
    schema = "vigilode.reaudit-bound-clients.v1" if bound_client else "vigilode.reaudit-comparison.v1"
    if row.get("schema") != schema:
        issues.append("ROW_SCHEMA")
    if not bound_client and row.get("suite") != case["suite"]:
        issues.append("ROW_MISMATCH:suite")
    for key, value in (("case", case["case"]), ("arm", arm), ("threads", threads),
                       ("scope", expected_scope), ("status", expected_status), ("full_curve_certified", full_curve)):
        if row.get(key) != value:
            issues.append("ROW_MISMATCH:" + key)
    if row.get("full_curve_certified") is not full_curve:
        issues.append("FULL_CURVE_FLAG_MISMATCH")
    bound, budget = row.get("achieved_bound"), row.get("original_budget")
    if any(type(x) not in (int, float) or not math.isfinite(x) for x in (bound, budget)):
        issues.append("BOUND_UNAVAILABLE")
    elif not (0 <= bound <= budget and budget > 0):
        issues.append("ORIGINAL_BUDGET_NOT_MET")
    if budget != case["original_budget"]:
        issues.append("FROZEN_BUDGET_CHANGED")
    if type(row.get("elapsed_ns")) is not int or row["elapsed_ns"] <= 0:
        issues.append("ELAPSED_UNAVAILABLE")
    identity = row.get("input_sha256", "")
    if not isinstance(identity, str) or len(identity) != 64 or any(c not in "0123456789abcdef" for c in identity):
        issues.append("INPUT_IDENTITY_INVALID")
    return issues


def memory_issues(receipt, config):
    process = receipt["process"]
    issues = []
    peak = process.get("peak_rss_bytes")
    if process.get("rss_status") != "OBSERVED" or type(peak) is not int or peak <= 0:
        issues.append("RSS_UNKNOWN")
    elif peak > config["memory_limit_bytes"]:
        issues.append("RSS_LIMIT_EXCEEDED")
    if process.get("affinity_escape") or not process.get("affinity_verified"):
        issues.append("AFFINITY_NOT_VERIFIED")
    oom = process.get("cgroup_oom_events_delta")
    if oom is not None and any(oom.values()):
        issues.append("CGROUP_OOM_EVENT_OBSERVED")
    return issues


def run_benchmarks(repo, output, source, config, cpus, resume):
    records, identities = [], {}
    deadline = time.monotonic() + config["invocation_timeout_seconds"]
    for case in config["comparison_cases"]:
        bound_client = case["suite"] == "bound-clients"
        binary = output / "targets/native/release/examples" / ("reaudit_bound_clients" if bound_client else "reaudit_comparison")
        for threads in config["threads"]:
            for repetition in range(-config["warmups"], config["repetitions"]):
                offset = (repetition + config["warmups"]) % len(case["arms"])
                arms = case["arms"][offset:] + case["arms"][:offset]
                for arm in arms:
                    if time.monotonic() >= deadline:
                        raise TimeoutError("invocation wall budget exhausted; receipts retained for resume")
                    phase = f"warmup{repetition + config['warmups']:02}" if repetition < 0 else f"repeat{repetition:02}"
                    identifier = f"{case['suite']}-{case['case']}-t{threads}-{phase}-{arm}"
                    argv = [str(binary), *([] if bound_client else ["--suite", case["suite"]]), "--case", case["case"],
                            "--arm", arm, "--threads", str(threads)]
                    receipt = execute(identifier, argv, repo, output, source, config, cpus[:threads], "native",
                                      config["comparison_timeout_seconds"], resume)
                    raw = output / "runs" / identifier / "stdout.log"
                    issues = memory_issues(receipt, config)
                    if receipt["status"] != "PASS":
                        issues.append("PROCESS_OR_SOURCE_FAILED")
                    try:
                        row = read_json(raw)
                        issues.extend(numeric_row_issues(row, case, arm, threads))
                        pair = (row.get("input_sha256"), row.get("original_budget"), row.get("scope"))
                        key = (case["suite"], case["case"])
                        if key in identities and identities[key] != pair:
                            issues.append("PAIRED_REQUEST_CHANGED")
                        identities.setdefault(key, pair)
                    except (ValueError, TypeError, AttributeError) as exc:
                        row = None
                        issues.append(f"INVALID_RAW_JSON:{exc}")
                    records.append({"id": identifier, "kind": "bound_client" if bound_client else "comparison", "case": case["case"],
                                    "suite": case["suite"], "arm": arm, "threads": threads,
                                    "phase": "warmup" if repetition < 0 else "measurement", "repetition": repetition,
                                    "status": "HOLD" if issues else "EXECUTED", "issues": issues,
                                    "process_elapsed_ns": process_elapsed_ns(receipt),
                                    "process_timing_scope": "external monotonic interval: launcher, process/threadpool startup, solver, certification, serialization, monitor cleanup",
                                    "raw_path": str(raw.relative_to(output)), "raw_sha256": digest(raw.read_bytes()),
                                    "receipt_path": f"runs/{identifier}/RECEIPT.json",
                                    "selected_backend_per_step": row.get("selected_backend_per_step") if row else None,
                                    "fallback_used": row.get("fallback_used") if row else None,
                                    "baseline_kind": row.get("baseline_kind") if row else None,
                                    "receipt_sha256": digest((output / "runs" / identifier / "RECEIPT.json").read_bytes())})
                    write_json(output / "BENCHMARK_CASES.json", records)
    raw = output / "WORK_PRECISION.json"
    bench = output / "targets/bench/release/vigilode-bench"
    argv = [str(bench), "wp", "--corpus", str(repo / config["corpus"]), "--out", str(raw),
            "--require-independent-reference", "--source-root", str(repo)]
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise TimeoutError("invocation wall budget exhausted before work precision")
    receipt = execute("work_precision", argv, repo, output, source, config, cpus[:1], "bench",
                      min(config["work_precision_timeout_seconds"], remaining), resume, raw_file=raw)
    issues = memory_issues(receipt, config)
    if receipt["status"] != "PASS":
        issues.append("PROCESS_OR_SOURCE_FAILED")
    try:
        read_json(raw)
    except (OSError, ValueError):
        issues.append("RAW_WORK_PRECISION_MISSING_OR_INVALID")
    records.append({"id": "work_precision", "kind": "work_precision", "status": "HOLD" if issues else "EXECUTED",
                    "issues": issues, "raw_path": "WORK_PRECISION.json",
                    "raw_sha256": digest(raw.read_bytes()) if raw.is_file() else None,
                    "receipt_path": "runs/work_precision/RECEIPT.json",
                    "receipt_sha256": digest((output / "runs/work_precision/RECEIPT.json").read_bytes()),
                    "process_elapsed_ns": process_elapsed_ns(receipt),
                    "memory_scope": "entire multi-arm process; no per-arm memory attribution",
                    "numerical_admission": "deferred to production_promotion_gate.py; exit zero is insufficient"})
    write_json(output / "BENCHMARK_CASES.json", records)
    return records


def process_elapsed_ns(receipt):
    elapsed = receipt["process"].get("elapsed_seconds")
    return round(elapsed * 1e9) if type(elapsed) in (int, float) and math.isfinite(elapsed) and elapsed > 0 else None


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("dry-run", "verify-only", "benchmark"), required=True)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--config", type=Path, default=Path("research/production_reaudit_20261021/NCP_CONFIG.json"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cpus", default="auto", help="allowed logical CPU list; auto takes first eight")
    parser.add_argument("--resume", action="store_true")
    args = parser.parse_args(argv)
    repo = args.repo.resolve()
    output = outside_output(repo, args.output)
    output.mkdir(parents=True, exist_ok=True)
    summary = {"schema": SCHEMA, "status": "HOLD", "mode": args.mode, "promotion": "NOT_EVALUATED",
               "issues": [], "validation": {"all_passed": False, "required_steps": [], "receipts": []},
               "benchmarks": {"cases": [], "all_executed": False}}
    try:
        config = read_json(args.config if args.config.is_absolute() else repo / args.config)
        validate_config(config, repo)
        cpus = choose_cpus(args.cpus, max(config["threads"]))
        source = source_identity(repo)
        pre = preflight(repo, output, config, source, args.mode == "dry-run")
        host, environment = host_identity(cpus), environment_identity()
        identity = {"source": source, "config_sha256": digest(canonical(config)), "host": host,
                    "environment": environment, "tools": pre["tools"], "packages": pre["packages"], "python": pre["python"],
                    "native_runtime": {"version": pre["native_probe"].get("runtime", {}).get("stdout"),
                                       "libraries": pre["native_probe"].get("runtime_libraries")}}
        summary.update({"source": source, "config_sha256": identity["config_sha256"], "host": host,
                        "environment": environment, "source_closure_policy": POLICY})
        summary["benchmarks"].update({"required_cases": planned_case_ids(config),
                                     "planned_comparison_cases": config["comparison_cases"], "threads": config["threads"],
                                     "warmups": config["warmups"], "repetitions": config["repetitions"],
                                     "work_precision_corpus": config["corpus"],
                                     "expected_comparison_rows": sum(len(c["arms"]) for c in config["comparison_cases"])
                                     * len(config["threads"]) * (config["warmups"] + config["repetitions"])})
        write_json(output / "PREFLIGHT.json", pre)
        plan = {"schema": SCHEMA, "identity": identity, "steps": steps(config), "config": config,
                "benchmark_protocol": "rotated paired arms, 2 warmups + 21 recorded repetitions at each thread count"}
        plan_path = output / "PLAN.json"
        if plan_path.is_file():
            old = read_json(plan_path)
            if not args.resume:
                raise ValueError("output already has a plan; use --resume or a new output directory")
            if old["identity"] != identity:
                raise ValueError("resume source/config/host/environment/tool identity changed")
        else:
            write_json(plan_path, plan)
        summary.update({"plan_path": "PLAN.json", "plan_sha256": digest(plan_path.read_bytes()),
                        "preflight_path": "PREFLIGHT.json", "preflight_sha256": digest((output / "PREFLIGHT.json").read_bytes())})
        config["_execution_identity_sha256"] = digest(canonical(identity))
        summary["execution_identity_sha256"] = config["_execution_identity_sha256"]
        summary["validation"]["required_steps"] = [step[0] for step in plan["steps"]]
        if pre["issues"]:
            summary["issues"].extend(pre["issues"])
        elif args.mode != "dry-run":
            for identifier, command, target in plan["steps"]:
                receipt = execute(identifier, command, repo, output, source, config, cpus[:1], target,
                                  config["validation_timeout_seconds"], args.resume)
                summary["validation"]["receipts"].append({"id": identifier, "status": receipt["status"],
                                                          "receipt_path": f"runs/{identifier}/RECEIPT.json",
                                                          "receipt_sha256": digest((output / "runs" / identifier / "RECEIPT.json").read_bytes())})
                if receipt["status"] != "PASS":
                    summary["issues"].append("VALIDATION_FAILED:" + identifier)
                    break
            summary["validation"]["all_passed"] = (
                len(summary["validation"]["receipts"]) == len(plan["steps"]) and not summary["issues"])
            if summary["validation"]["all_passed"] and args.mode == "benchmark":
                summary["benchmarks"]["cases"] = run_benchmarks(repo, output, source, config, cpus, args.resume)
                rows = summary["benchmarks"]["cases"]
                summary["benchmarks"]["all_executed"] = (
                    all(row["status"] == "EXECUTED" for row in rows)
                    and len(rows) == len(summary["benchmarks"]["required_cases"])
                    and {row["id"] for row in rows} == set(summary["benchmarks"]["required_cases"]))
                if not summary["benchmarks"]["all_executed"]:
                    summary["issues"].append("BENCHMARK_HAS_HOLDS")
            if source_identity(repo) != source:
                summary["issues"].append("SOURCE_CHANGED_DURING_EXECUTION")
        if not summary["issues"]:
            summary["status"] = {"dry-run": "PLANNED", "verify-only": "VERIFIED", "benchmark": "BENCHMARK_EXECUTED"}[args.mode]
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, TimeoutError, subprocess.SubprocessError) as exc:
        summary["issues"].append(f"{type(exc).__name__}:{exc}")
        if (output / "BENCHMARK_CASES.json").is_file():
            try:
                summary["benchmarks"]["cases"] = read_json(output / "BENCHMARK_CASES.json")
            except (OSError, ValueError):
                pass
    write_json(output / "SUMMARY.json", summary)
    print(json.dumps({"status": summary["status"], "summary": str(output / "SUMMARY.json"), "issues": summary["issues"]}))
    return 0 if summary["status"] != "HOLD" else 2


if __name__ == "__main__":
    raise SystemExit(main())
