#!/usr/bin/env python3
"""Fail-closed timing-authority validation for the sealed VigilODE v3.7 contract.

This module does not run the solver and does not produce timing measurements.  It
captures host/source attestations, validates already-created complete campaign
directories, summarizes at most five retained attempts, and produces a
retrospective diagnostic for the consumed v3.6 timing evidence.
"""

from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import math
import os
import platform
import re
import statistics
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence


SCHEMA = "vigilode-v37-timing-replication-continuation-transaction-contract-v1"
ATTESTATION_SCHEMA = "vigilode-v37-timing-host-attestation-v1"
CAMPAIGN_DECISION_SCHEMA = "vigilode-v37-timing-campaign-decision-v2"
ATTEMPT_SUMMARY_SCHEMA = "vigilode-v37-timing-attempt-summary-v2"
AUTHORITY_EVIDENCE_SCHEMA = "vigilode-v37-timing-authority-evidence-v1"
RETROSPECTIVE_SCHEMA = "vigilode-v37-v36-retrospective-timing-quality-v1"
PASS_VERDICT = "PASS_HOST_QUALIFIED_DESCRIPTIVE_TIMING"
FAIL_VERDICT = "NON_AUTHORITY_HOST_QUALITY_FAIL"
INSUFFICIENT_VERDICT = "HOST_UNSUITABLE_NO_TIMING_PROMOTION"
V36_RETROSPECTIVE_VERDICT = "WHOLE_V36_CAMPAIGN_NON_AUTHORITY_DUE_TO_N384_HOST_QUALITY_FAILURE"

PROFILE_FILES = {
    96: "calibration96.json",
    192: "calibration192.json",
    256: "calibration256.json",
    320: "holdout320.json",
    384: "holdout384.json",
}
PROFILE_LABELS = {
    96: "stage-growth-calibration-96",
    192: "stage-growth-calibration-192",
    256: "stage-growth-calibration-256",
    320: "enforced-budget-holdout-320",
    384: "stage-growth-holdout-384",
}
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
GIT_SHA_RE = re.compile(r"^[0-9a-f]{40}$")


class ValidationError(RuntimeError):
    """Raised for malformed or incomplete authority inputs."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def _finite_number(value: Any, name: str, *, positive: bool = False) -> float:
    require(isinstance(value, (int, float)) and not isinstance(value, bool), f"{name} must be numeric")
    number = float(value)
    require(math.isfinite(number), f"{name} must be finite")
    if positive:
        require(number > 0.0, f"{name} must be positive")
    return number


def _exact_keys(obj: Mapping[str, Any], required: Iterable[str], where: str, *, allow_extra: bool = True) -> None:
    missing = sorted(set(required) - set(obj))
    require(not missing, f"{where} missing keys: {missing}")
    if not allow_extra:
        extra = sorted(set(obj) - set(required))
        require(not extra, f"{where} has extra keys: {extra}")


def sha256_path(path: Path | str) -> str:
    path = Path(path)
    require(path.is_file(), f"file missing for SHA-256: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def sha256_tree(root: Path | str) -> str:
    root = Path(root)
    require(root.is_dir(), f"directory missing for SHA-256: {root}")
    digest = hashlib.sha256()
    files = sorted(path for path in root.rglob("*") if path.is_file())
    for path in files:
        rel = path.relative_to(root).as_posix().encode("utf-8")
        digest.update(len(rel).to_bytes(8, "big"))
        digest.update(rel)
        file_hash = bytes.fromhex(sha256_path(path))
        digest.update(file_hash)
    return digest.hexdigest()


def _canonical_json_bytes(obj: Any) -> bytes:
    return json.dumps(
        obj,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def _canonical_json_sha256(obj: Any) -> str:
    return hashlib.sha256(_canonical_json_bytes(obj)).hexdigest()


def _canonical_float(value: Any) -> float:
    number = float(value)
    return 0.0 if number == 0.0 else number


def load_json(path: Path | str) -> Any:
    path = Path(path)
    require(path.is_file(), f"JSON file missing: {path}")
    try:
        with path.open("r", encoding="utf-8") as stream:
            return json.load(stream)
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise ValidationError(f"invalid JSON {path}: {exc}") from exc


def _validate_contract_exact(contract: Mapping[str, Any]) -> None:
    require(contract.get("schema") == SCHEMA, "timing contract schema mismatch")
    timing = contract.get("timing_replication")
    require(isinstance(timing, dict), "timing_replication object missing")
    expected: dict[str, Any] = {
        "profiles": [96, 192, 256, 320, 384],
        "families_per_profile": 6,
        "warmup_pairs_per_profile": 1,
        "measured_pairs_per_profile_per_campaign": 7,
        "required_passing_campaigns": 3,
        "maximum_campaign_attempts": 5,
        "preflight_probe_seconds": 10,
        "minimum_cpu_idle_fraction": 0.9,
        "maximum_cpu_steal_fraction": 0.001,
        "maximum_swap_in_delta": 0,
        "maximum_swap_out_delta": 0,
        "maximum_exposed_thermal_throttle_counter_delta": 0,
        "maximum_rjf_arm_wall_span": 1.5,
        "maximum_shadow_arm_wall_span": 1.5,
        "maximum_order_median_absolute_gap": 0.1,
        "authority_exclusion_unit": "whole-campaign",
        "individual_pair_exclusion_allowed": False,
        "individual_profile_exclusion_allowed": False,
        "all_pairs_retained": True,
        "all_failed_campaigns_retained": True,
        "quality_rules_reference_ratio_direction": False,
        "require_clean_tree": True,
        "require_measurement_profile_attestation": True,
        "require_exact_paired_proposed_interval": True,
        "require_same_git_identity": True,
        "require_same_rust_toolchain": True,
        "require_same_measurement_binary_sha256": True,
        "require_same_host_cpu_kernel_fingerprint": True,
        "require_same_cpu_affinity": True,
        "require_same_thread_environment": True,
        "speedup_claim_authorized": False,
        "thermal_counter_absence_alone_is_failure": False,
        "failed_campaign_verdict": FAIL_VERDICT,
        "insufficient_passing_campaigns_verdict": INSUFFICIENT_VERDICT,
        "proc_stat_total_fields": [
            "user",
            "nice",
            "system",
            "idle",
            "iowait",
            "irq",
            "softirq",
            "steal",
        ],
        "host_fingerprint_fields": [
            "kernel",
            "cpu_model",
            "logical_cpu_count",
            "physical_core_count",
            "microcode",
            "numa_node_count",
            "frequency_governor",
            "boost_or_turbo_state",
        ],
        "thread_environment_fields": [
            "RAYON_NUM_THREADS",
            "OMP_NUM_THREADS",
            "OPENBLAS_NUM_THREADS",
            "MKL_NUM_THREADS",
            "BLIS_NUM_THREADS",
            "VECLIB_MAXIMUM_THREADS",
            "NUMEXPR_NUM_THREADS",
        ],
    }
    for key, value in expected.items():
        require(timing.get(key) == value, f"sealed timing contract mismatch: {key}")


def load_contract(path: Path | str) -> dict[str, Any]:
    path = Path(path).resolve()
    obj = load_json(path)
    require(isinstance(obj, dict), "contract root must be an object")
    _validate_contract_exact(obj)
    result = dict(obj)
    result["_authority"] = {"path": str(path), "sha256": sha256_path(path)}
    return result


def parse_proc_stat_cpu(text: str) -> tuple[int, ...]:
    line = next((line for line in text.splitlines() if line.startswith("cpu ")), None)
    require(line is not None, "aggregate cpu line missing")
    fields = line.split()[1:9]
    require(len(fields) == 8, "sealed cpu field count mismatch")
    try:
        values = tuple(int(value) for value in fields)
    except ValueError as exc:
        raise ValidationError("aggregate cpu fields must be integers") from exc
    require(all(value >= 0 for value in values), "aggregate cpu fields must be nonnegative")
    return values


def cpu_idle_steal_fractions(before: Sequence[int], after: Sequence[int]) -> tuple[float, float]:
    require(len(before) == 8 and len(after) == 8, "sealed cpu field count mismatch")
    delta = tuple(int(end) - int(start) for start, end in zip(before, after))
    require(all(value >= 0 for value in delta), "invalid negative cpu delta")
    total = sum(delta)
    require(total > 0, "cpu total delta is zero")
    idle = delta[3] / total
    steal = delta[7] / total
    require(math.isfinite(idle) and math.isfinite(steal), "nonfinite cpu fraction")
    return idle, steal


def _run(args: Sequence[str], *, cwd: Path | None = None) -> str:
    try:
        proc = subprocess.run(
            list(args),
            cwd=str(cwd) if cwd else None,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    except OSError as exc:
        raise ValidationError(f"command unavailable: {args[0]}: {exc}") from exc
    require(proc.returncode == 0, f"command failed ({proc.returncode}): {' '.join(args)}: {proc.stderr.strip()}")
    return proc.stdout.strip()


def _read_text(path: Path | str) -> str:
    path = Path(path)
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        raise ValidationError(f"cannot read {path}: {exc}") from exc


def _parse_cpuinfo(text: str) -> dict[str, Any]:
    blocks = [block for block in text.split("\n\n") if block.strip()]
    entries: list[dict[str, str]] = []
    for block in blocks:
        item: dict[str, str] = {}
        for line in block.splitlines():
            if ":" in line:
                key, value = line.split(":", 1)
                item[key.strip()] = value.strip()
        if item:
            entries.append(item)
    first = entries[0] if entries else {}
    model = first.get("model name") or first.get("Hardware") or first.get("Processor") or platform.processor()
    microcode = first.get("microcode")
    cores = {
        (entry.get("physical id"), entry.get("core id"))
        for entry in entries
        if entry.get("physical id") is not None and entry.get("core id") is not None
    }
    physical = len(cores) if cores else None
    return {"cpu_model": model or "UNKNOWN", "microcode": microcode, "physical_core_count": physical}


def _numa_node_count() -> int | None:
    root = Path("/sys/devices/system/node")
    if not root.is_dir():
        return None
    nodes = [path for path in root.glob("node[0-9]*") if path.is_dir()]
    return len(nodes) if nodes else None


def _first_existing_text(paths: Sequence[Path]) -> str | None:
    for path in paths:
        if path.is_file():
            try:
                value = path.read_text(encoding="utf-8", errors="replace").strip()
            except OSError:
                continue
            if value:
                return value
    return None


def capture_host_fingerprint() -> dict[str, Any]:
    cpuinfo = _parse_cpuinfo(_read_text("/proc/cpuinfo"))
    governor_paths = sorted(Path("/sys/devices/system/cpu").glob("cpu[0-9]*/cpufreq/scaling_governor"))
    governors = []
    for path in governor_paths:
        try:
            value = path.read_text(encoding="utf-8", errors="replace").strip()
        except OSError:
            continue
        if value:
            governors.append(value)
    governor = ",".join(sorted(set(governors))) if governors else None
    turbo = _first_existing_text(
        [
            Path("/sys/devices/system/cpu/intel_pstate/no_turbo"),
            Path("/sys/devices/system/cpu/cpufreq/boost"),
        ]
    )
    if turbo is not None:
        if Path("/sys/devices/system/cpu/intel_pstate/no_turbo").is_file():
            turbo = "disabled" if turbo == "1" else "enabled" if turbo == "0" else turbo
        else:
            turbo = "enabled" if turbo == "1" else "disabled" if turbo == "0" else turbo
    return {
        "kernel": _run(["uname", "-srmo"]),
        "cpu_model": cpuinfo["cpu_model"],
        "logical_cpu_count": os.cpu_count(),
        "physical_core_count": cpuinfo["physical_core_count"],
        "microcode": cpuinfo["microcode"],
        "numa_node_count": _numa_node_count(),
        "frequency_governor": governor,
        "boost_or_turbo_state": turbo,
    }


def allowed_affinity() -> list[int]:
    try:
        affinity = sorted(int(cpu) for cpu in os.sched_getaffinity(0))
    except (AttributeError, OSError) as exc:
        raise ValidationError(f"cpu affinity unavailable: {exc}") from exc
    require(bool(affinity), "cpu affinity is empty")
    return affinity


def thread_environment(fields: Sequence[str]) -> dict[str, str | None]:
    return {field: os.environ.get(field) for field in fields}


def _capture_swap_counters() -> dict[str, int]:
    text = _read_text("/proc/vmstat")
    values: dict[str, int] = {}
    for line in text.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[0] in {"pswpin", "pswpout"}:
            try:
                values[parts[0]] = int(parts[1])
            except ValueError as exc:
                raise ValidationError(f"invalid /proc/vmstat {parts[0]}") from exc
    require(set(values) == {"pswpin", "pswpout"}, "swap counters missing")
    return values


def _capture_thermal_counters() -> dict[str, int]:
    patterns = [
        "/sys/devices/system/cpu/cpu*/thermal_throttle/core_throttle_count",
        "/sys/devices/system/cpu/cpu*/thermal_throttle/package_throttle_count",
    ]
    values: dict[str, int] = {}
    for pattern in patterns:
        for path in sorted(Path("/").glob(pattern.lstrip("/"))):
            try:
                raw = path.read_text(encoding="utf-8", errors="replace").strip()
                values[str(path)] = int(raw)
            except (OSError, ValueError):
                continue
    return values


def _counter_delta(before: Mapping[str, int], after: Mapping[str, int], name: str) -> dict[str, int]:
    require(set(before) == set(after), f"{name} counter set changed during preflight")
    delta: dict[str, int] = {}
    for key in sorted(before):
        value = int(after[key]) - int(before[key])
        require(value >= 0, f"negative {name} counter delta: {key}")
        delta[key] = value
    return delta


def capture_preflight(seconds: int) -> dict[str, Any]:
    require(isinstance(seconds, int) and seconds > 0, "preflight seconds must be positive integer")
    cpu_before = parse_proc_stat_cpu(_read_text("/proc/stat"))
    swap_before = _capture_swap_counters()
    thermal_before = _capture_thermal_counters()
    time.sleep(seconds)
    cpu_after = parse_proc_stat_cpu(_read_text("/proc/stat"))
    swap_after = _capture_swap_counters()
    thermal_after = _capture_thermal_counters()
    idle, steal = cpu_idle_steal_fractions(cpu_before, cpu_after)
    return {
        "probe_seconds": seconds,
        "cpu_before": list(cpu_before),
        "cpu_after": list(cpu_after),
        "cpu_idle_fraction": idle,
        "cpu_steal_fraction": steal,
        "swap_before": swap_before,
        "swap_after": swap_after,
        "swap_delta": _counter_delta(swap_before, swap_after, "swap"),
        "thermal_before": thermal_before,
        "thermal_after": thermal_after,
        "thermal_delta": _counter_delta(thermal_before, thermal_after, "thermal"),
    }


def capture_attestation(
    repo_root: Path | str,
    binary: Path | str,
    contract: Path | str,
    preflight_seconds: int,
) -> dict[str, Any]:
    repo_root = Path(repo_root).resolve()
    binary = Path(binary).resolve()
    contract_path = Path(contract).resolve()
    loaded = load_contract(contract_path)
    require(
        preflight_seconds == loaded["timing_replication"]["preflight_probe_seconds"],
        "preflight duration must equal sealed contract",
    )
    require(repo_root.is_dir(), f"repository root missing: {repo_root}")
    require(binary.is_file(), f"measurement binary missing: {binary}")
    require(any(a == "target" and b == "measurement" for a, b in zip(binary.parts, binary.parts[1:])), "measurement binary path must contain target/measurement")
    head = _run(["git", "rev-parse", "HEAD"], cwd=repo_root)
    tree = _run(["git", "rev-parse", "HEAD^{tree}"], cwd=repo_root)
    dirty = _run(["git", "status", "--porcelain"], cwd=repo_root)
    require(GIT_SHA_RE.fullmatch(head) is not None, "invalid Git HEAD")
    require(GIT_SHA_RE.fullmatch(tree) is not None, "invalid Git tree")
    result = {
        "schema": ATTESTATION_SCHEMA,
        "git": {"head": head, "tree": tree, "clean": dirty == ""},
        "rust": {
            "rustc_vv": _run(["rustc", "-Vv"]),
            "cargo_version": _run(["cargo", "--version"]),
        },
        "contract_sha256": loaded["_authority"]["sha256"],
        "binary_sha256": sha256_path(binary),
        "measurement_profile": "measurement",
        "host": capture_host_fingerprint(),
        "cpu_affinity": allowed_affinity(),
        "thread_environment": thread_environment(loaded["timing_replication"]["thread_environment_fields"]),
        "preflight": capture_preflight(preflight_seconds),
    }
    return result


def _validate_hash(value: Any, name: str, regex: re.Pattern[str]) -> str:
    require(isinstance(value, str) and regex.fullmatch(value) is not None, f"invalid {name}")
    return value


def _attestation_structural_validation(contract: Mapping[str, Any], attestation: Mapping[str, Any]) -> None:
    timing = contract["timing_replication"]
    require(attestation.get("schema") == ATTESTATION_SCHEMA, "attestation schema mismatch")
    _exact_keys(attestation, ["git", "rust", "contract_sha256", "binary_sha256", "measurement_profile", "host", "cpu_affinity", "thread_environment", "preflight"], "attestation")
    git = attestation["git"]
    rust = attestation["rust"]
    host = attestation["host"]
    preflight = attestation["preflight"]
    require(isinstance(git, dict) and isinstance(rust, dict) and isinstance(host, dict) and isinstance(preflight, dict), "attestation nested object malformed")
    _validate_hash(git.get("head"), "Git HEAD", GIT_SHA_RE)
    _validate_hash(git.get("tree"), "Git tree", GIT_SHA_RE)
    require(isinstance(git.get("clean"), bool), "Git clean flag must be boolean")
    require(isinstance(rust.get("rustc_vv"), str) and rust["rustc_vv"].strip(), "rustc identity missing")
    require(isinstance(rust.get("cargo_version"), str) and rust["cargo_version"].strip(), "cargo identity missing")
    _validate_hash(attestation.get("contract_sha256"), "contract SHA-256", SHA256_RE)
    _validate_hash(attestation.get("binary_sha256"), "binary SHA-256", SHA256_RE)
    require(attestation.get("measurement_profile") == "measurement", "measurement profile attestation mismatch")
    require(set(timing["host_fingerprint_fields"]) <= set(host), "host fingerprint fields missing")
    require(isinstance(host.get("kernel"), str) and host["kernel"], "kernel fingerprint missing")
    require(isinstance(host.get("cpu_model"), str) and host["cpu_model"], "CPU model fingerprint missing")
    require(
        isinstance(host.get("logical_cpu_count"), int)
        and not isinstance(host["logical_cpu_count"], bool)
        and host["logical_cpu_count"] > 0,
        "logical CPU count invalid",
    )
    physical_cores = host.get("physical_core_count")
    require(
        physical_cores is None
        or (
            isinstance(physical_cores, int)
            and not isinstance(physical_cores, bool)
            and physical_cores > 0
        ),
        "physical core count invalid",
    )
    numa_nodes = host.get("numa_node_count")
    require(
        numa_nodes is None
        or (
            isinstance(numa_nodes, int)
            and not isinstance(numa_nodes, bool)
            and numa_nodes > 0
        ),
        "NUMA node count invalid",
    )
    require(
        host.get("microcode") is None
        or (isinstance(host["microcode"], str) and bool(host["microcode"])),
        "microcode fingerprint invalid",
    )
    for field in ("frequency_governor", "boost_or_turbo_state"):
        require(
            host.get(field) is None
            or (isinstance(host[field], str) and bool(host[field])),
            f"host fingerprint field invalid: {field}",
        )
    affinity = attestation.get("cpu_affinity")
    require(
        isinstance(affinity, list)
        and affinity
        and all(
            isinstance(cpu, int) and not isinstance(cpu, bool) and cpu >= 0
            for cpu in affinity
        )
        and affinity == sorted(set(affinity)),
        "CPU affinity invalid",
    )
    env = attestation.get("thread_environment")
    require(isinstance(env, dict), "thread environment missing")
    require(set(env) == set(timing["thread_environment_fields"]), "thread environment field set mismatch")
    require(all(value is None or isinstance(value, str) for value in env.values()), "thread environment values invalid")
    _exact_keys(preflight, ["probe_seconds", "cpu_before", "cpu_after", "cpu_idle_fraction", "cpu_steal_fraction", "swap_before", "swap_after", "swap_delta", "thermal_before", "thermal_after", "thermal_delta"], "preflight")
    require(
        isinstance(preflight.get("probe_seconds"), int)
        and not isinstance(preflight["probe_seconds"], bool)
        and preflight["probe_seconds"] == timing["preflight_probe_seconds"],
        "preflight duration mismatch",
    )
    for name in ("cpu_before", "cpu_after"):
        values = preflight.get(name)
        require(
            isinstance(values, list)
            and len(values) == 8
            and all(
                isinstance(value, int)
                and not isinstance(value, bool)
                and value >= 0
                for value in values
            ),
            f"{name} must contain the sealed eight nonnegative integer fields",
        )
    before = tuple(preflight["cpu_before"])
    after = tuple(preflight["cpu_after"])
    computed_idle, computed_steal = cpu_idle_steal_fractions(before, after)
    stated_idle = _finite_number(preflight.get("cpu_idle_fraction"), "cpu_idle_fraction")
    stated_steal = _finite_number(preflight.get("cpu_steal_fraction"), "cpu_steal_fraction")
    require(math.isclose(stated_idle, computed_idle, rel_tol=0.0, abs_tol=1e-15), "cpu idle fraction mismatch")
    require(math.isclose(stated_steal, computed_steal, rel_tol=0.0, abs_tol=1e-15), "cpu steal fraction mismatch")
    for name in ("swap_before", "swap_after", "swap_delta", "thermal_before", "thermal_after", "thermal_delta"):
        counters = preflight.get(name)
        require(isinstance(counters, dict), f"{name} must be an object")
        require(
            all(
                isinstance(key, str)
                and bool(key)
                and isinstance(value, int)
                and not isinstance(value, bool)
                and value >= 0
                for key, value in counters.items()
            ),
            f"{name} counters must be nonnegative integers",
        )
    require(set(preflight["swap_before"]) == {"pswpin", "pswpout"}, "swap before fields mismatch")
    require(set(preflight["swap_after"]) == {"pswpin", "pswpout"}, "swap after fields mismatch")
    require(set(preflight["swap_delta"]) == {"pswpin", "pswpout"}, "swap delta fields mismatch")
    expected_swap = _counter_delta(preflight["swap_before"], preflight["swap_after"], "swap")
    require(preflight["swap_delta"] == expected_swap, "swap delta mismatch")
    expected_thermal = _counter_delta(preflight["thermal_before"], preflight["thermal_after"], "thermal")
    require(preflight["thermal_delta"] == expected_thermal, "thermal delta mismatch")


def _attestation_quality_failures(contract: Mapping[str, Any], attestation: Mapping[str, Any]) -> list[dict[str, Any]]:
    timing = contract["timing_replication"]
    preflight = attestation["preflight"]
    failures: list[dict[str, Any]] = []

    def fail(name: str, actual: Any, threshold: Any) -> None:
        failures.append({"name": name, "actual": actual, "threshold": threshold})

    if timing["require_clean_tree"] and not attestation["git"]["clean"]:
        fail("clean-tree", False, True)
    if attestation["contract_sha256"] != contract["_authority"]["sha256"]:
        fail("contract-hash", attestation["contract_sha256"], contract["_authority"]["sha256"])
    if preflight["cpu_idle_fraction"] < timing["minimum_cpu_idle_fraction"]:
        fail("cpu-idle", preflight["cpu_idle_fraction"], f">={timing['minimum_cpu_idle_fraction']}")
    if preflight["cpu_steal_fraction"] > timing["maximum_cpu_steal_fraction"]:
        fail("cpu-steal", preflight["cpu_steal_fraction"], f"<={timing['maximum_cpu_steal_fraction']}")
    if preflight["swap_delta"]["pswpin"] > timing["maximum_swap_in_delta"]:
        fail("swap-in", preflight["swap_delta"]["pswpin"], timing["maximum_swap_in_delta"])
    if preflight["swap_delta"]["pswpout"] > timing["maximum_swap_out_delta"]:
        fail("swap-out", preflight["swap_delta"]["pswpout"], timing["maximum_swap_out_delta"])
    for path, delta in sorted(preflight["thermal_delta"].items()):
        if delta > timing["maximum_exposed_thermal_throttle_counter_delta"]:
            fail("thermal-throttle", {"path": path, "delta": delta}, timing["maximum_exposed_thermal_throttle_counter_delta"])
    return failures


def _validate_arm(
    arm: Mapping[str, Any],
    *,
    mode: str,
    families: int,
    repetitions: int,
    where: str,
) -> dict[str, Any]:
    require(isinstance(arm, dict), f"{where} arm must be an object")
    require(arm.get("mode") == mode, f"{where} mode mismatch")
    require(
        isinstance(arm.get("repetitions"), int)
        and not isinstance(arm["repetitions"], bool)
        and arm["repetitions"] == repetitions,
        f"{where} repetition count mismatch",
    )
    wall = _finite_number(arm.get("wall_seconds"), f"{where} wall_seconds", positive=True)
    interval = _finite_number(arm.get("proposed_interval"), f"{where} proposed_interval", positive=True)
    gamma = _finite_number(arm.get("gamma_seconds_per_interval"), f"{where} gamma", positive=True)
    require(math.isclose(gamma, wall / interval, rel_tol=5e-15, abs_tol=1e-15), f"{where} Gamma formula mismatch")
    require(arm.get("family_count") == families, f"{where} family count mismatch")
    require(arm.get("all_suite_identities_passed") is True, f"{where} suite identity failed")
    return {"wall_seconds": wall, "proposed_interval": interval, "gamma_seconds_per_interval": gamma}


def _validate_pair(
    row: Mapping[str, Any],
    index: int,
    timing: Mapping[str, Any],
    repetitions: int,
    where: str,
) -> dict[str, Any]:
    require(isinstance(row, dict), f"{where} pair row must be an object")
    require(row.get("pair_index") == index, f"{where} pair index mismatch")
    expected_order = "rjf-first" if index % 2 == 0 else "shadow-first"
    require(row.get("order") == expected_order, f"{where} order mismatch")
    rjf = _validate_arm(
        row.get("rjf_only"),
        mode="rjf-only",
        families=timing["families_per_profile"],
        repetitions=repetitions,
        where=f"{where}.rjf",
    )
    shadow = _validate_arm(
        row.get("frozen_full_e_shadow"),
        mode="frozen-full-e-shadow",
        families=timing["families_per_profile"],
        repetitions=repetitions,
        where=f"{where}.shadow",
    )
    pair_failures: list[dict[str, Any]] = []
    if rjf["proposed_interval"] != shadow["proposed_interval"]:
        pair_failures.append(
            {
                "name": "proposed-interval",
                "where": where,
                "rjf": rjf["proposed_interval"],
                "shadow": shadow["proposed_interval"],
            }
        )
    ratio = _finite_number(row.get("wall_ratio_shadow_over_rjf"), f"{where} wall ratio", positive=True)
    gamma_ratio = _finite_number(row.get("gamma_ratio_shadow_over_rjf"), f"{where} gamma ratio", positive=True)
    computed_ratio = shadow["wall_seconds"] / rjf["wall_seconds"]
    require(math.isclose(ratio, computed_ratio, rel_tol=5e-15, abs_tol=1e-15), f"{where} wall ratio formula mismatch")
    require(math.isclose(gamma_ratio, shadow["gamma_seconds_per_interval"] / rjf["gamma_seconds_per_interval"], rel_tol=5e-15, abs_tol=1e-15), f"{where} gamma ratio formula mismatch")
    return {
        "pair_index": index,
        "order": expected_order,
        "rjf_wall_seconds": rjf["wall_seconds"],
        "shadow_wall_seconds": shadow["wall_seconds"],
        "proposed_interval": rjf["proposed_interval"],
        "wall_ratio_shadow_over_rjf": ratio,
        "quality_failures": pair_failures,
        "raw": row,
    }


def _profile_metrics(rows: Sequence[Mapping[str, Any]]) -> dict[str, float | None]:
    if not rows:
        return {
            "rjf_arm_wall_span": None,
            "shadow_arm_wall_span": None,
            "order_median_absolute_gap": None,
            "median_shadow_over_rjf": None,
        }
    rjf = [float(row["rjf_wall_seconds"]) for row in rows]
    shadow = [float(row["shadow_wall_seconds"]) for row in rows]
    ratios = [float(row["wall_ratio_shadow_over_rjf"]) for row in rows]
    rjf_first = [ratio for row, ratio in zip(rows, ratios) if row["order"] == "rjf-first"]
    shadow_first = [ratio for row, ratio in zip(rows, ratios) if row["order"] == "shadow-first"]
    return {
        "rjf_arm_wall_span": max(rjf) / min(rjf),
        "shadow_arm_wall_span": max(shadow) / min(shadow),
        "order_median_absolute_gap": (
            abs(statistics.median(rjf_first) - statistics.median(shadow_first))
            if rjf_first and shadow_first
            else None
        ),
        "median_shadow_over_rjf": statistics.median(ratios),
    }


def _profile_quality_failures(
    timing: Mapping[str, Any], dim: int, metrics: Mapping[str, float | None]
) -> list[dict[str, Any]]:
    failures: list[dict[str, Any]] = []
    unavailable = sorted(name for name, value in metrics.items() if value is None)
    if unavailable:
        failures.append(
            {
                "name": "profile-metric-unavailable",
                "dimension": dim,
                "metrics": unavailable,
                "reason": "insufficient-retained-pairs",
            }
        )
    if (
        metrics["rjf_arm_wall_span"] is not None
        and metrics["rjf_arm_wall_span"] > timing["maximum_rjf_arm_wall_span"]
    ):
        failures.append({"name": "rjf-arm-span", "dimension": dim, "actual": metrics["rjf_arm_wall_span"], "threshold": timing["maximum_rjf_arm_wall_span"]})
    if (
        metrics["shadow_arm_wall_span"] is not None
        and metrics["shadow_arm_wall_span"] > timing["maximum_shadow_arm_wall_span"]
    ):
        failures.append({"name": "shadow-arm-span", "dimension": dim, "actual": metrics["shadow_arm_wall_span"], "threshold": timing["maximum_shadow_arm_wall_span"]})
    if (
        metrics["order_median_absolute_gap"] is not None
        and metrics["order_median_absolute_gap"] > timing["maximum_order_median_absolute_gap"]
    ):
        failures.append({"name": "order-median-gap", "dimension": dim, "actual": metrics["order_median_absolute_gap"], "threshold": timing["maximum_order_median_absolute_gap"]})
    return failures


def _validate_retained_pair_rows(
    rows: Sequence[Mapping[str, Any]],
    *,
    required_count: int,
    timing: Mapping[str, Any],
    dim: int,
    kind: str,
    repetitions: int,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    retained: list[dict[str, Any]] = []
    indices: list[int] = []
    for position, row in enumerate(rows):
        require(isinstance(row, Mapping), f"profile-{dim}.{kind}.retained-{position} row must be an object")
        declared_index = row.get("pair_index")
        require(
            isinstance(declared_index, int) and not isinstance(declared_index, bool),
            f"profile-{dim}.{kind}.retained-{position} pair index must be an integer",
        )
        retained.append(
            _validate_pair(
                row,
                declared_index,
                timing,
                repetitions,
                f"profile-{dim}.{kind}.retained-{position}",
            )
        )
        indices.append(declared_index)

    counts = Counter(indices)
    expected = set(range(required_count))
    actual = set(indices)
    missing = sorted(expected - actual)
    unexpected = sorted(actual - expected)
    duplicates = sorted(index for index, count in counts.items() if count > 1)
    failures: list[dict[str, Any]] = []
    if missing or unexpected or duplicates:
        failures.append(
            {
                "name": "profile-pair-index-set",
                "dimension": dim,
                "kind": kind,
                "missing": missing,
                "unexpected": unexpected,
                "duplicates": duplicates,
                "retained_indices": indices,
                "required_indices": list(range(required_count)),
            }
        )
    return retained, failures


def _load_profile_file(path: Path, dim: int, contract: Mapping[str, Any]) -> dict[str, Any]:
    timing = contract["timing_replication"]
    data = load_json(path)
    require(isinstance(data, dict), f"profile {dim} root must be an object")
    require(data.get("schema") == "g4-s5b0-frozen-full-e-shadow-economics-v1", f"profile {dim} schema mismatch")
    require(data.get("status") == "complete", f"profile {dim} incomplete")
    require(data.get("profile") == PROFILE_LABELS[dim], f"profile {dim} label mismatch")
    require(data.get("switching_active") is False, f"profile {dim} switching must be false")
    require(data.get("committed_method") == "protected-sequential-matrix-free-rodas5p", f"profile {dim} committed method mismatch")
    require(data.get("shadow_method") == "pexprb54s4-fused-resume-retained-level2", f"profile {dim} shadow method mismatch")
    require(
        data.get("frozen_zeta34_tau") == contract["frozen_policy"]["zeta34_tau"],
        f"profile {dim} frozen zeta34 tau mismatch",
    )
    require(data.get("all_six_families_present") is True, f"profile {dim} family coverage mismatch")
    paired = data.get("paired_wall")
    require(isinstance(paired, dict), f"profile {dim} paired_wall missing")
    require(paired.get("required_build_profile") == "measurement", f"profile {dim} required build profile mismatch")
    require(paired.get("measurement_build_verified") is True, f"profile {dim} measurement build unverified")
    require(paired.get("compiled_profile_directory") == "measurement", f"profile {dim} compiled directory mismatch")
    require(paired.get("compiled_cargo_profile") == "release", f"profile {dim} raw Cargo profile mismatch")
    require(paired.get("suite_scope") == "all-six-families", f"profile {dim} suite scope mismatch")
    require(paired.get("calibration_arm") == "rjf-only", f"profile {dim} calibration arm mismatch")
    require(paired.get("gamma_denominator") == "sum-absolute-proposed-attempt-h", f"profile {dim} Gamma denominator mismatch")
    require(paired.get("all_suite_identities_passed") is True, f"profile {dim} suite identity failed")
    frozen_repetitions = paired.get("frozen_repetitions")
    require(
        isinstance(frozen_repetitions, int)
        and not isinstance(frozen_repetitions, bool)
        and frozen_repetitions > 0,
        f"profile {dim} frozen repetition count invalid",
    )
    warmups = paired.get("warmup_rows")
    measured = paired.get("measured_rows")
    require(isinstance(warmups, list), f"profile {dim} warmup rows must be a list")
    require(isinstance(measured, list), f"profile {dim} measured rows must be a list")
    failures: list[dict[str, Any]] = []
    declared_warmups = paired.get("warmup_pairs")
    declared_measured = paired.get("measured_pairs")
    require(
        isinstance(declared_warmups, int) and not isinstance(declared_warmups, bool),
        f"profile {dim} declared warmup pair count must be an integer",
    )
    require(
        isinstance(declared_measured, int) and not isinstance(declared_measured, bool),
        f"profile {dim} declared measured pair count must be an integer",
    )
    if declared_warmups != timing["warmup_pairs_per_profile"] or len(warmups) != timing["warmup_pairs_per_profile"]:
        failures.append(
            {
                "name": "profile-pair-cardinality",
                "dimension": dim,
                "kind": "warmup",
                "declared": declared_warmups,
                "retained": len(warmups),
                "required": timing["warmup_pairs_per_profile"],
            }
        )
    if declared_measured != timing["measured_pairs_per_profile_per_campaign"] or len(measured) != timing["measured_pairs_per_profile_per_campaign"]:
        failures.append(
            {
                "name": "profile-pair-cardinality",
                "dimension": dim,
                "kind": "measured",
                "declared": declared_measured,
                "retained": len(measured),
                "required": timing["measured_pairs_per_profile_per_campaign"],
            }
        )
    warmup_rows, warmup_index_failures = _validate_retained_pair_rows(
        warmups,
        required_count=timing["warmup_pairs_per_profile"],
        timing=timing,
        dim=dim,
        kind="warmup",
        repetitions=frozen_repetitions,
    )
    measured_rows, measured_index_failures = _validate_retained_pair_rows(
        measured,
        required_count=timing["measured_pairs_per_profile_per_campaign"],
        timing=timing,
        dim=dim,
        kind="measured",
        repetitions=frozen_repetitions,
    )
    failures.extend(warmup_index_failures)
    failures.extend(measured_index_failures)
    for row in warmup_rows + measured_rows:
        failures.extend(row["quality_failures"])
    metrics = _profile_metrics(measured_rows)
    failures.extend(_profile_quality_failures(timing, dim, metrics))
    return {
        "dimension": dim,
        "path": str(path),
        "sha256": sha256_path(path),
        "warmup_rows": warmup_rows,
        "measured_rows": measured_rows,
        "metrics": metrics,
        "quality_failures": failures,
        "quality_failure_names": sorted({failure["name"] for failure in failures}),
    }


def _comparison_identity(attestation: Mapping[str, Any]) -> dict[str, Any]:
    return {
        "git": attestation["git"],
        "rust": attestation["rust"],
        "contract_sha256": attestation["contract_sha256"],
        "binary_sha256": attestation["binary_sha256"],
        "host": attestation["host"],
        "cpu_affinity": attestation["cpu_affinity"],
        "thread_environment": attestation["thread_environment"],
    }


def _validate_exact_campaign_layout(root: Path) -> tuple[Path, Path]:
    expected_root = {"ATTESTATION.json", "profiles"}
    root_entries = {path.name: path for path in root.iterdir()}
    require(
        set(root_entries) == expected_root,
        f"campaign root entry set mismatch: expected={sorted(expected_root)} actual={sorted(root_entries)}",
    )
    attestation_path = root_entries["ATTESTATION.json"]
    profiles_root = root_entries["profiles"]
    require(
        attestation_path.is_file() and not attestation_path.is_symlink(),
        "ATTESTATION.json must be a regular file",
    )
    require(
        profiles_root.is_dir() and not profiles_root.is_symlink(),
        "profiles must be a regular directory",
    )
    expected_profiles = set(PROFILE_FILES.values())
    profile_entries = {path.name: path for path in profiles_root.iterdir()}
    require(
        set(profile_entries) == expected_profiles,
        f"profile entry set mismatch: expected={sorted(expected_profiles)} actual={sorted(profile_entries)}",
    )
    for name, path in profile_entries.items():
        require(path.is_file() and not path.is_symlink(), f"profile entry must be a regular file: {name}")
    return attestation_path, profiles_root


def _preflight_authority_evidence(preflight: Mapping[str, Any]) -> dict[str, Any]:
    cpu_before = tuple(int(value) for value in preflight["cpu_before"])
    cpu_after = tuple(int(value) for value in preflight["cpu_after"])
    idle, steal = cpu_idle_steal_fractions(cpu_before, cpu_after)
    swap_before = {key: int(value) for key, value in preflight["swap_before"].items()}
    swap_after = {key: int(value) for key, value in preflight["swap_after"].items()}
    thermal_before = {
        key: int(value) for key, value in preflight["thermal_before"].items()
    }
    thermal_after = {
        key: int(value) for key, value in preflight["thermal_after"].items()
    }
    return {
        "probe_seconds": int(preflight["probe_seconds"]),
        "cpu_before": list(cpu_before),
        "cpu_after": list(cpu_after),
        "cpu_idle_fraction": _canonical_float(idle),
        "cpu_steal_fraction": _canonical_float(steal),
        "swap_before": swap_before,
        "swap_after": swap_after,
        "swap_delta": _counter_delta(swap_before, swap_after, "swap"),
        "thermal_before": thermal_before,
        "thermal_after": thermal_after,
        "thermal_delta": _counter_delta(thermal_before, thermal_after, "thermal"),
    }


def _attestation_authority_evidence(
    contract: Mapping[str, Any], attestation: Mapping[str, Any]
) -> dict[str, Any]:
    timing = contract["timing_replication"]
    return {
        "schema": attestation["schema"],
        "git": {
            "head": attestation["git"]["head"],
            "tree": attestation["git"]["tree"],
            "clean": attestation["git"]["clean"],
        },
        "rust": {
            "rustc_vv": attestation["rust"]["rustc_vv"],
            "cargo_version": attestation["rust"]["cargo_version"],
        },
        "contract_sha256": attestation["contract_sha256"],
        "binary_sha256": attestation["binary_sha256"],
        "measurement_profile": attestation["measurement_profile"],
        "host": {
            field: attestation["host"].get(field)
            for field in timing["host_fingerprint_fields"]
        },
        "cpu_affinity": list(attestation["cpu_affinity"]),
        "thread_environment": {
            field: attestation["thread_environment"].get(field)
            for field in timing["thread_environment_fields"]
        },
        "preflight": _preflight_authority_evidence(attestation["preflight"]),
    }


def _arm_authority_evidence(arm: Mapping[str, Any]) -> dict[str, Any]:
    return {
        "mode": arm["mode"],
        "repetitions": int(arm["repetitions"]),
        "wall_seconds": _canonical_float(arm["wall_seconds"]),
        "proposed_interval": _canonical_float(arm["proposed_interval"]),
        "family_count": int(arm["family_count"]),
        "all_suite_identities_passed": bool(arm["all_suite_identities_passed"]),
    }


def _pair_authority_evidence(parsed_row: Mapping[str, Any]) -> dict[str, Any]:
    raw = parsed_row["raw"]
    return {
        "pair_index": int(raw["pair_index"]),
        "order": raw["order"],
        "rjf_only": _arm_authority_evidence(raw["rjf_only"]),
        "frozen_full_e_shadow": _arm_authority_evidence(raw["frozen_full_e_shadow"]),
    }


def _sorted_pair_authority_evidence(
    rows: Sequence[Mapping[str, Any]],
) -> list[dict[str, Any]]:
    evidence = [_pair_authority_evidence(row) for row in rows]
    return sorted(
        evidence,
        key=lambda item: (
            item["pair_index"],
            json.dumps(
                item,
                sort_keys=True,
                separators=(",", ":"),
                ensure_ascii=False,
                allow_nan=False,
            ),
        ),
    )


def _campaign_authority_evidence_sha256(
    contract: Mapping[str, Any],
    attestation: Mapping[str, Any],
    profiles: Sequence[Mapping[str, Any]],
) -> str:
    payload = {
        "schema": AUTHORITY_EVIDENCE_SCHEMA,
        "attestation": _attestation_authority_evidence(contract, attestation),
        "profiles": [
            {
                "dimension": profile["dimension"],
                "profile_label": PROFILE_LABELS[profile["dimension"]],
                "warmup_rows": _sorted_pair_authority_evidence(
                    profile["warmup_rows"]
                ),
                "measured_rows": _sorted_pair_authority_evidence(
                    profile["measured_rows"]
                ),
            }
            for profile in profiles
        ],
    }
    return _canonical_json_sha256(payload)


def validate_campaign(contract: Mapping[str, Any], campaign_root: Path | str) -> dict[str, Any]:
    _validate_contract_exact(contract)
    require("_authority" in contract, "contract authority metadata missing; use load_contract")
    root = Path(campaign_root).resolve()
    require(root.is_dir(), f"campaign directory missing: {root}")
    attestation_path, profiles_root = _validate_exact_campaign_layout(root)
    attestation = load_json(attestation_path)
    require(isinstance(attestation, dict), "attestation root must be an object")
    _attestation_structural_validation(contract, attestation)
    profiles = [_load_profile_file(profiles_root / PROFILE_FILES[dim], dim, contract) for dim in contract["timing_replication"]["profiles"]]
    failures = _attestation_quality_failures(contract, attestation)
    for profile in profiles:
        failures.extend(profile["quality_failures"])
    names = sorted({failure["name"] for failure in failures})
    return {
        "schema": CAMPAIGN_DECISION_SCHEMA,
        "campaign_path": str(root),
        "campaign_sha256": sha256_tree(root),
        "authority_evidence_sha256": _campaign_authority_evidence_sha256(
            contract, attestation, profiles
        ),
        "attestation_path": str(attestation_path),
        "attestation_sha256": sha256_path(attestation_path),
        "comparison_identity": _comparison_identity(attestation),
        "profiles": profiles,
        "retained_profile_count": len(profiles),
        "retained_warmup_pair_count": sum(len(profile["warmup_rows"]) for profile in profiles),
        "retained_measured_pair_count": sum(len(profile["measured_rows"]) for profile in profiles),
        "quality_failures": failures,
        "quality_failure_names": names,
        "verdict": PASS_VERDICT if not failures else FAIL_VERDICT,
        "timing_authority": not failures,
        "speedup_claim_authorized": False,
        "active_switching_authorized": False,
        "individual_pair_exclusion_used": False,
        "individual_profile_exclusion_used": False,
        "ratio_direction_used_for_quality": False,
    }


def _identity_token(identity: Mapping[str, Any]) -> str:
    payload = json.dumps(identity, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _identity_failure_names(reference: Mapping[str, Any], current: Mapping[str, Any]) -> list[str]:
    comparisons = [
        ("git-identity", "git"),
        ("rust-toolchain", "rust"),
        ("contract-hash", "contract_sha256"),
        ("measurement-binary", "binary_sha256"),
        ("host-fingerprint", "host"),
        ("cpu-affinity", "cpu_affinity"),
        ("thread-environment", "thread_environment"),
    ]
    return [name for name, key in comparisons if reference.get(key) != current.get(key)]


def _validate_campaign_decision_for_summary(
    contract: Mapping[str, Any], raw: Any, attempt_index: int
) -> dict[str, Any]:
    where = f"attempt {attempt_index}"
    require(isinstance(raw, Mapping), f"{where} result must be an object")
    campaign_path = raw.get("campaign_path")
    require(isinstance(campaign_path, str) and campaign_path, f"{where} campaign path missing")
    canonical = validate_campaign(contract, campaign_path)
    require(
        _canonical_json_bytes(dict(raw)) == _canonical_json_bytes(canonical),
        f"{where} decision does not exactly match revalidated campaign evidence",
    )
    return {
        "raw": canonical,
        "campaign_path": canonical["campaign_path"],
        "normalized_campaign_path": str(
            Path(canonical["campaign_path"]).expanduser().resolve(strict=False)
        ),
        "campaign_sha256": canonical["campaign_sha256"],
        "authority_evidence_sha256": canonical["authority_evidence_sha256"],
        "identity": canonical["comparison_identity"],
        "quality_failure_names": list(canonical["quality_failure_names"]),
    }


def summarize_attempts(contract: Mapping[str, Any], attempt_results: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    _validate_contract_exact(contract)
    require("_authority" in contract, "contract authority metadata missing; use load_contract")
    timing = contract["timing_replication"]
    require(isinstance(attempt_results, Sequence) and not isinstance(attempt_results, (str, bytes)), "attempt results must be a sequence")
    require(1 <= len(attempt_results) <= timing["maximum_campaign_attempts"], "attempt count outside sealed range")
    retained: list[dict[str, Any]] = []
    reference_token: str | None = None
    reference_identity: Mapping[str, Any] | None = None
    seen_campaign_paths: set[str] = set()
    seen_campaign_hashes: set[str] = set()
    seen_authority_evidence_hashes: set[str] = set()
    passing = 0
    for index, raw in enumerate(attempt_results, start=1):
        validated = _validate_campaign_decision_for_summary(contract, raw, index)
        require(
            validated["normalized_campaign_path"] not in seen_campaign_paths,
            f"attempt {index} duplicates a retained campaign path",
        )
        require(
            validated["campaign_sha256"] not in seen_campaign_hashes,
            f"attempt {index} duplicates a retained campaign tree SHA-256",
        )
        require(
            validated["authority_evidence_sha256"] not in seen_authority_evidence_hashes,
            f"attempt {index} duplicates retained timing authority evidence",
        )
        seen_campaign_paths.add(validated["normalized_campaign_path"])
        seen_campaign_hashes.add(validated["campaign_sha256"])
        seen_authority_evidence_hashes.add(validated["authority_evidence_sha256"])
        identity = validated["identity"]
        token = _identity_token(identity)
        if reference_token is None:
            reference_token = token
            reference_identity = identity
        effective_failures = list(validated["quality_failure_names"])
        if identity["contract_sha256"] != contract["_authority"]["sha256"]:
            effective_failures.append("contract-hash")
        if token != reference_token:
            require(reference_identity is not None, "reference identity missing")
            effective_failures.extend(_identity_failure_names(reference_identity, identity))
        effective_failures = sorted(set(effective_failures))
        effective_pass = validated["raw"]["verdict"] == PASS_VERDICT and not effective_failures
        if effective_pass:
            passing += 1
        retained.append(
            {
                "attempt_index": index,
                "campaign_path": validated["campaign_path"],
                "campaign_sha256": validated["campaign_sha256"],
                "authority_evidence_sha256": validated["authority_evidence_sha256"],
                "original_verdict": validated["raw"]["verdict"],
                "effective_verdict": PASS_VERDICT if effective_pass else FAIL_VERDICT,
                "effective_quality_failures": effective_failures,
                "comparison_identity_sha256": token,
            }
        )
    verdict = PASS_VERDICT if passing >= timing["required_passing_campaigns"] else INSUFFICIENT_VERDICT
    return {
        "schema": ATTEMPT_SUMMARY_SCHEMA,
        "verdict": verdict,
        "attempt_count": len(retained),
        "passing_campaign_count": passing,
        "required_passing_campaigns": timing["required_passing_campaigns"],
        "maximum_campaign_attempts": timing["maximum_campaign_attempts"],
        "attempts": retained,
        "all_attempts_retained": True,
        "speedup_claim_authorized": False,
        "active_switching_authorized": False,
        "individual_pair_exclusion_used": False,
        "individual_profile_exclusion_used": False,
    }


def generate_v36_retrospective_diagnostic(contract: Mapping[str, Any], economics_root: Path | str) -> dict[str, Any]:
    _validate_contract_exact(contract)
    root = Path(economics_root).resolve()
    require(root.is_dir(), f"v3.6 economics directory missing: {root}")
    actual = {path.name for path in root.glob("*.json")}
    expected = set(PROFILE_FILES.values())
    require(actual == expected, f"v3.6 economics file set mismatch: expected={sorted(expected)} actual={sorted(actual)}")
    profiles: list[dict[str, Any]] = []
    all_failures: list[dict[str, Any]] = []
    timing = contract["timing_replication"]
    for dim in timing["profiles"]:
        path = root / PROFILE_FILES[dim]
        profile = _load_profile_file(path, dim, contract)
        profiles.append(
            {
                "dimension": dim,
                "path": PROFILE_FILES[dim],
                "sha256": profile["sha256"],
                "measured_pair_count": len(profile["measured_rows"]),
                "metrics": profile["metrics"],
                "quality_failures": profile["quality_failures"],
                "quality_failure_names": profile["quality_failure_names"],
            }
        )
        all_failures.extend(profile["quality_failures"])
    require(any(profile["dimension"] == 384 and "rjf-arm-span" in profile["quality_failure_names"] for profile in profiles), "expected N=384 R-JF span failure absent")
    require(any(profile["dimension"] == 384 and "shadow-arm-span" in profile["quality_failure_names"] for profile in profiles), "expected N=384 shadow span failure absent")
    return {
        "schema": RETROSPECTIVE_SCHEMA,
        "analysis_role": "CONTRACT_DIAGNOSTIC_ONLY_NOT_NEW_TIMING_AUTHORITY",
        "historical_verdict": "PASS_DESCRIPTIVE_ECONOMICS",
        "historical_verdict_rewritten": False,
        "retrospective_verdict": V36_RETROSPECTIVE_VERDICT,
        "profiles": profiles,
        "retained_profile_count": len(profiles),
        "retained_measured_pair_count": sum(profile["measured_pair_count"] for profile in profiles),
        "quality_failures": all_failures,
        "quality_failure_names": sorted({failure["name"] for failure in all_failures}),
        "historical_host_counters": {
            "status": "NOT_RECORDED",
            "cpu_idle_fraction": None,
            "cpu_steal_fraction": None,
            "swap_delta": None,
            "thermal_delta": None,
        },
        "all_pairs_retained": True,
        "individual_pair_exclusion_used": False,
        "individual_profile_exclusion_used": False,
        "speedup_claim_authorized": False,
        "active_switching_authorized": False,
    }


def atomic_write_json(path: Path | str, obj: Any) -> None:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    try:
        with temporary.open("w", encoding="utf-8", newline="\n") as stream:
            json.dump(obj, stream, indent=2, sort_keys=True, ensure_ascii=False, allow_nan=False)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        directory_fd: int | None = None
        try:
            directory_fd = os.open(path.parent, os.O_RDONLY)
            os.fsync(directory_fd)
        except OSError:
            pass
        finally:
            if directory_fd is not None:
                os.close(directory_fd)
    except Exception:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise


def _command_capture(args: argparse.Namespace) -> dict[str, Any]:
    return capture_attestation(args.repo, args.binary, args.contract, args.preflight_seconds)


def _command_validate(args: argparse.Namespace) -> dict[str, Any]:
    return validate_campaign(load_contract(args.contract), args.campaign_root)


def _command_summarize(args: argparse.Namespace) -> dict[str, Any]:
    attempts = [load_json(path) for path in args.attempt_result]
    return summarize_attempts(load_contract(args.contract), attempts)


def _command_retrospective(args: argparse.Namespace) -> dict[str, Any]:
    return generate_v36_retrospective_diagnostic(load_contract(args.contract), args.economics_root)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    capture = sub.add_parser("capture", help="capture immutable host/source attestation")
    capture.add_argument("--repo", type=Path, required=True)
    capture.add_argument("--binary", type=Path, required=True)
    capture.add_argument("--contract", type=Path, required=True)
    capture.add_argument("--preflight-seconds", type=int, default=10)
    capture.add_argument("--output", type=Path, required=True)
    capture.set_defaults(handler=_command_capture)

    validate = sub.add_parser("validate-campaign", help="validate one complete campaign directory")
    validate.add_argument("--contract", type=Path, required=True)
    validate.add_argument("--campaign-root", type=Path, required=True)
    validate.add_argument("--output", type=Path, required=True)
    validate.set_defaults(handler=_command_validate)

    summarize = sub.add_parser("summarize", help="summarize retained campaign decisions")
    summarize.add_argument("--contract", type=Path, required=True)
    summarize.add_argument("--attempt-result", type=Path, action="append", required=True)
    summarize.add_argument("--output", type=Path, required=True)
    summarize.set_defaults(handler=_command_summarize)

    retrospective = sub.add_parser("retrospective-v36", help="generate deterministic v3.6 diagnostic")
    retrospective.add_argument("--contract", type=Path, required=True)
    retrospective.add_argument("--economics-root", type=Path, required=True)
    retrospective.add_argument("--output", type=Path, required=True)
    retrospective.set_defaults(handler=_command_retrospective)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        result = args.handler(args)
        atomic_write_json(args.output, result)
        return 0
    except ValidationError as exc:
        print(f"validation error: {exc}", file=sys.stderr)
        return 1
    except (OSError, ValueError, TypeError, json.JSONDecodeError) as exc:
        print(f"execution error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
