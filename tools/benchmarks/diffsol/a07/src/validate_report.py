#!/usr/bin/env python3
"""Capture immutable identities and validate A07 evidence before reporting."""
import argparse
import collections
import hashlib
import json
import math
import os
import pathlib
import platform
import random
import statistics
import struct
import subprocess
import sys

import scipy

ROOT = pathlib.Path(__file__).resolve().parents[5]
EVIDENCE = ROOT / "research/production_audit_20261018/evidence/continuation_a07"
THRESHOLD = 0.05


def sha(data):
    return hashlib.sha256(data).hexdigest()


def command(args, cwd=ROOT):
    return subprocess.run(args, cwd=cwd, text=True, capture_output=True, check=False)


def capture_manifest(args):
    cargo_tree = command(["cargo", "tree", "--locked", "--offline", "--edges", "normal"], cwd=ROOT / "tools/benchmarks/diffsol/a07")
    cargo_tree_features = command(["cargo", "tree", "--locked", "--offline", "--edges", "features"], cwd=ROOT / "tools/benchmarks/diffsol/a07")
    metadata = command(["cargo", "metadata", "--locked", "--offline", "--no-deps", "--format-version", "1"], cwd=ROOT / "tools/benchmarks/diffsol/a07")
    status = command(["git", "status", "--porcelain=v1", "--branch"])
    head = command(["git", "rev-parse", "HEAD"])
    branch = command(["git", "branch", "--show-current"])
    diff = command(["git", "diff", "--binary", "HEAD"])
    rustc = command(["rustc", "-Vv"])
    cargo = command(["cargo", "-V"])
    uname = command(["uname", "-a"])
    lscpu = command(["lscpu"])
    tracked = command(["git", "ls-files", "crates/rodas5p-core", "crates/rodas5p-integrators", "tools/benchmarks/diffsol/a07"])
    source_paths = set(tracked.stdout.splitlines())
    harness = ROOT / "tools/benchmarks/diffsol/a07"
    source_paths.update(
        str(path.relative_to(ROOT))
        for path in harness.rglob("*")
        if path.is_file() and "target" not in path.parts
    )
    source_entries = {}
    for rel in sorted(source_paths):
        path = ROOT / rel
        if path.is_file() and (rel.endswith(".rs") or rel.endswith(".py") or rel.endswith(".toml") or rel.endswith("Cargo.lock")):
            source_entries[rel] = sha(path.read_bytes())
    binary_entries = {}
    for label, filename in (("timing", args.timing_binary), ("allocator", args.allocator_binary)):
        if filename:
            path = pathlib.Path(filename).resolve()
            binary_entries[label] = {"path": str(path), "sha256": sha(path.read_bytes()) if path.is_file() else None}
            if binary_entries[label]["sha256"] is None:
                raise RuntimeError(f"required {label} binary does not exist: {path}")
    config_entries = {}
    for rel in (".cargo/config", ".cargo/config.toml", "rust-toolchain", "rust-toolchain.toml"):
        path = ROOT / rel
        if path.is_file():
            config_entries[rel] = sha(path.read_bytes())
    doc = {
        "schema": "vigilode-a07-source-manifest-v1",
        "repository": str(ROOT),
        "branch": branch.stdout.strip(),
        "head": head.stdout.strip(),
        "git_status": status.stdout,
        "git_dirty": bool(status.stdout.splitlines()[1:]),
        "git_diff_binary_sha256": sha(diff.stdout.encode()),
        "git_diff_binary_exit_code": diff.returncode,
        "rustc_vv": rustc.stdout,
        "cargo_version": cargo.stdout.strip(),
        "target": platform.platform(),
        "machine": platform.machine(),
        "processor": platform.processor(),
        "logical_cpu_count": os.cpu_count(),
        "uname": uname.stdout.strip(),
        "lscpu": lscpu.stdout,
        "python": platform.python_version(),
        "scipy": scipy.__version__,
        "environment": {k: os.environ.get(k) for k in ("RUSTFLAGS", "CARGO_PROFILE", "RAYON_NUM_THREADS", "OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS")},
        "build_profiles": {"timing": "measurement", "allocator": "measurement + allocation-audit"},
        "build_commands": [
            "cargo build --locked --offline --profile measurement",
            "cargo build --locked --offline --profile measurement --features allocation-audit",
        ],
        "measurement_instrumentation": "Timed binaries compile out callback counters and contain no callback counter atomics or branches; timing rows record callback_counters=null with reason callback counters disabled to preserve timing path. A separate allocation-audit build/run uses callback counters and global allocation instrumentation with the same equations, tolerances, and grid; those counts are outside timing and are not used to adjust timing rows.",
        "binary_sha256": binary_entries,
        "source_sha256": source_entries,
        "config_sha256": config_entries,
        "cargo_tree_exit_code": cargo_tree.returncode,
        "cargo_tree_sha256": sha(cargo_tree.stdout.encode()),
        "cargo_tree": cargo_tree.stdout,
        "cargo_tree_stderr": cargo_tree.stderr,
        "cargo_tree_features_exit_code": cargo_tree_features.returncode,
        "cargo_tree_features_sha256": sha(cargo_tree_features.stdout.encode()),
        "cargo_tree_features": cargo_tree_features.stdout,
        "cargo_metadata_exit_code": metadata.returncode,
        "cargo_metadata_no_deps_sha256": sha(metadata.stdout.encode()),
        "standalone_lock_sha256": sha((ROOT / "tools/benchmarks/diffsol/a07/Cargo.lock").read_bytes()),
        "grid_file_sha256": sha(pathlib.Path(args.grid).read_bytes()),
        "reference_file_sha256": sha(pathlib.Path(args.reference).read_bytes()),
    }
    if "diffsol v0.16.1" not in cargo_tree.stdout or "diffsol v0.16.1" not in cargo_tree_features.stdout:
        raise RuntimeError("resolved dependency graph does not contain pinned Diffsol 0.16.1")
    if cargo_tree.returncode or cargo_tree_features.returncode or metadata.returncode:
        raise RuntimeError("locked offline cargo identity commands failed")
    output = pathlib.Path(args.out)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(doc, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")


def load_jsonl(paths):
    rows = []

    def reject_constant(value):
        raise ValueError(f"non-standard JSON numeric constant {value}")

    for filename in paths:
        with open(filename, encoding="utf-8") as stream:
            for lineno, line in enumerate(stream, 1):
                try:
                    row = json.loads(line, parse_constant=reject_constant)
                    if not isinstance(row, dict):
                        rows.append({"status": "invalid_json", "error": "JSONL value is not an object", "raw_json_value": row,
                                     "evidence_file": filename, "evidence_line": lineno})
                        continue
                    row["evidence_file"] = filename
                    row["evidence_line"] = lineno
                    rows.append(row)
                except (json.JSONDecodeError, ValueError) as exc:
                    rows.append({"status": "invalid_json", "error": str(exc), "evidence_file": filename, "evidence_line": lineno})
    return rows


def normalized_wrms(state, reference, atol, rtol):
    values = []
    for value, ref in zip(state, reference):
        scale = atol + rtol * abs(ref)
        values.append(((value - ref) / scale) ** 2)
    return (sum(values) / len(values)) ** 0.5


def validate_reference_center(reference, grid):
    center = reference.get("reference_center")
    ladder = reference.get("radau_ladder")
    if not isinstance(ladder, list) or not ladder:
        raise RuntimeError("reference lacks a Radau ladder")
    fine_states = ladder[-1].get("states") if isinstance(ladder[-1], dict) else None

    def valid_matrix(states):
        return (
            isinstance(states, list)
            and len(states) == len(grid)
            and all(
                isinstance(row, list)
                and len(row) == 3
                and all(
                    isinstance(value, (int, float))
                    and not isinstance(value, bool)
                    and math.isfinite(value)
                    for value in row
                )
                for row in states
            )
        )

    if not valid_matrix(center):
        raise RuntimeError("reference_center must be a finite 101x3 state matrix")
    if not valid_matrix(fine_states):
        raise RuntimeError("fine Radau ladder state matrix must be finite 101x3")
    if center != fine_states:
        raise RuntimeError("reference_center does not exactly equal the final Radau ladder states")


def errors(rows, reference, grid):
    fine = reference["reference_center"]
    spreads = reference["reference_spreads_empirical_not_certified"]
    uncertainty = max(spreads.values())
    result = {"reference_spreads": spreads, "reference_uncertainty_empirical_not_certified": uncertainty,
              "reference_gate": all(x <= THRESHOLD for x in spreads.values()), "arms": {}}
    for arm in ("native", "diffsol"):
        arm_rows = [row for row in rows if row.get("arm") == arm]
        valid = [row for row in arm_rows if row.get("status") == "ok" and len(row.get("states", [])) == len(grid)]
        distances = [max(normalized_wrms(y, r, 1.0e-11, 1.0e-8) for y, r in zip(row["states"], fine)) for row in valid]
        max_distance = max(distances) if distances else None
        result["arms"][arm] = {
            "rows": len(arm_rows), "successful_complete_rows": len(valid), "failed_rows": len(arm_rows) - len(valid),
            "max_grid_wrms_distance_to_fine_radau": max_distance,
            "empirical_interval": [max(0.0, max_distance - uncertainty), max_distance + uncertainty] if max_distance is not None else None,
            "accuracy_gate": bool(max_distance is not None and max_distance + uncertainty <= 1.0 and len(valid) == len(arm_rows)),
            "work_counter_receipts": [row.get("work_counters") for row in valid],
            "callback_counter_receipts": [row.get("callback_counters") for row in valid],
            "errors_and_partial_rows": [row for row in arm_rows if row not in valid],
        }
    ni = result["arms"]["native"]["empirical_interval"]
    di = result["arms"]["diffsol"]["empirical_interval"]
    overlap = bool(ni and di and max(ni[0], di[0]) <= min(ni[1], di[1]))
    result["arm_intervals_overlap"] = overlap
    result["accuracy_overlap_gate"] = bool(result["reference_gate"] and overlap and all(v["accuracy_gate"] for v in result["arms"].values()))
    return result


def validate_campaign_rows(rows):
    required = {
        "schema", "arm", "protocol", "status", "elapsed_ns", "allocations", "allocated_bytes",
        "output_count", "times", "states", "work_counters", "solver_counters",
        "unavailable_counters_reason", "partial_solver_counters_reason", "error", "timing_authority", "grid_sha256", "manifest_sha256", "binary_sha256", "callback_counters",
    }
    allowed_protocols = {"single_cold_process", "warm_abba", "separate_allocator_run"}
    allowed_extra = {"pair_id", "within_pair_order", "process_exit_code", "evidence_file", "evidence_line"}
    issues = []
    for index, row in enumerate(rows):
        if not isinstance(row, dict):
            issues.append({"row": index, "issue": "row_not_object"})
            continue
        missing = sorted(required - row.keys())
        if missing:
            issues.append({"row": index, "issue": "missing_fields", "fields": missing})
            continue
        unknown = sorted(row.keys() - required - allowed_extra)
        if unknown:
            issues.append({"row": index, "issue": "unknown_fields", "fields": unknown})
        if row["schema"] != "vigilode-a07-run-v1":
            issues.append({"row": index, "issue": "unknown_schema", "value": row["schema"]})
        if row["arm"] not in {"native", "diffsol"}:
            issues.append({"row": index, "issue": "unknown_arm", "value": row["arm"]})
        if row["protocol"] not in allowed_protocols:
            issues.append({"row": index, "issue": "unknown_protocol", "value": row["protocol"]})
        if row["status"] not in {"ok", "failed", "process_failed", "output_grid_mismatch", "invalid_output", "identity_mismatch", "binary_identity_mismatch"}:
            issues.append({"row": index, "issue": "unknown_status", "value": row["status"]})
        if row["timing_authority"] != "diagnostic_only":
            issues.append({"row": index, "issue": "timing_authority_mismatch"})
        if not isinstance(row["grid_sha256"], str) or not isinstance(row["manifest_sha256"], str):
            issues.append({"row": index, "issue": "missing_identity"})
        if row["protocol"] in {"single_cold_process", "warm_abba"} and (
            not isinstance(row["elapsed_ns"], int) or row["elapsed_ns"] <= 0
        ):
            issues.append({"row": index, "issue": "missing_timing"})
        if row["protocol"] == "single_cold_process":
            if not isinstance(row.get("process_exit_code"), int) or row.get("process_exit_code") != 0:
                issues.append({"row": index, "issue": "cold_child_exit", "value": row.get("process_exit_code")})
            if not isinstance(row.get("pair_id"), int) or not isinstance(row.get("within_pair_order"), int):
                issues.append({"row": index, "issue": "cold_pair_metadata_missing"})
        if row["protocol"] == "separate_allocator_run" and (
            not isinstance(row["allocations"], int) or not isinstance(row["allocated_bytes"], int)
        ):
            issues.append({"row": index, "issue": "missing_allocator_measurement"})
        if row["protocol"] in {"single_cold_process", "warm_abba"}:
            if row["callback_counters"] is not None:
                issues.append({"row": index, "issue": "timing_callback_counters_must_be_null"})
            if row["unavailable_counters_reason"] != "callback counters disabled to preserve timing path":
                issues.append({"row": index, "issue": "timing_callback_reason_mismatch"})
        elif row["protocol"] == "separate_allocator_run":
            counters = row["callback_counters"]
            expected_counter_fields = {
                "rhs_calls", "rhs_batch_calls", "rhs_evaluations", "jacobian_builds",
                "jacobian_into_calls", "jvp_calls", "jvp_vectors",
            }
            if not isinstance(counters, dict) or set(counters) != expected_counter_fields or any(
                not isinstance(value, int) or value < 0 for value in counters.values()
            ):
                issues.append({"row": index, "issue": "allocator_callback_counts_missing_or_malformed"})
    cold = [r for r in rows if r.get("protocol") == "single_cold_process"]
    warm = [r for r in rows if r.get("protocol") == "warm_abba"]
    alloc = [r for r in rows if r.get("protocol") == "separate_allocator_run"]
    if len(cold) != 20:
        issues.append({"issue": "cold_count", "expected": 20, "actual": len(cold)})
    else:
        for pair_id in range(10):
            pair = [r for r in cold if r.get("pair_id") == pair_id]
            expected = ["native", "diffsol"] if pair_id % 2 == 0 else ["diffsol", "native"]
            ordered = sorted(pair, key=lambda r: r.get("within_pair_order", -1))
            if len(pair) != 2 or [r.get("arm") for r in ordered] != expected or [r.get("within_pair_order") for r in ordered] != [0, 1]:
                issues.append({"issue": "cold_pair_protocol", "pair_id": pair_id})
    if len(warm) != 160:
        issues.append({"issue": "warm_count", "expected": 160, "actual": len(warm)})
    else:
        sequence = ["native", "diffsol", "diffsol", "native", "diffsol", "native", "native", "diffsol"]
        for block in range(20):
            block_rows = warm[block * 8 : (block + 1) * 8]
            if [r.get("arm") for r in block_rows] != sequence:
                issues.append({"issue": "warm_abba_sequence", "block": block})
    if len(alloc) != 2 or collections.Counter(r.get("arm") for r in alloc) != collections.Counter(["diffsol", "native"]):
        issues.append({"issue": "allocator_arm_counts", "expected": 2, "actual": len(alloc)})
    return {"valid": not issues, "issues": issues, "counts": {"cold": len(cold), "warm": len(warm), "allocator": len(alloc)}}


def timing_summary(rows, protocol):
    protocol_rows = [r for r in rows if r.get("protocol") == protocol]
    if protocol == "warm_abba":
        for i, row in enumerate(protocol_rows):
            row.setdefault("pair_id", i // 8)
            row.setdefault("within_pair_order", i % 8)
    selected = [r for r in protocol_rows if r.get("status") == "ok" and isinstance(r.get("elapsed_ns"), (int, float))]
    pairs = {}
    for row in selected:
        pair_id = row.get("pair_id")
        if pair_id is None or row.get("arm") not in {"native", "diffsol"}:
            continue
        pairs.setdefault(pair_id, {"native": [], "diffsol": []})[row["arm"]].append(row["elapsed_ns"])
    ratios = []
    for pair in pairs.values():
        expected_each = 4 if protocol == "warm_abba" else 1
        if len(pair["native"]) == expected_each and len(pair["diffsol"]) == expected_each:
            native = statistics.median(pair["native"])
            diffsol = statistics.median(pair["diffsol"])
            ratios.append(diffsol / native)
    median_ratio = statistics.median(ratios) if ratios else None
    ci = None
    if len(ratios) >= 2:
        rng = random.Random(20261018)
        samples = []
        for _ in range(10000):
            draw = [ratios[rng.randrange(len(ratios))] for _ in ratios]
            samples.append(statistics.median(draw))
        samples.sort()
        ci = [samples[249], samples[9749]]
    return {"protocol": protocol, "successful_rows": len(selected), "failed_rows": sum(r.get("status") != "ok" for r in protocol_rows),
            "paired_units": len(ratios), "diffsol_over_native_median_ratio": median_ratio,
            "bootstrap_95pct_interval": ci, "bootstrap_resamples": 10000 if ci else 0,
            "timing_authority": "diagnostic_only"}


def validate(args):
    grid_doc = json.loads(pathlib.Path(args.grid).read_text())
    reference = json.loads(pathlib.Path(args.reference).read_text())
    manifest = json.loads(pathlib.Path(args.manifest).read_text())
    grid = grid_doc["times"]
    canonical_times_hash = sha(json.dumps(grid, separators=(",", ":"), allow_nan=False).encode())
    if len(grid) != 101 or grid[0] != 0.0 or grid[-1] != 40.0 or any(t != 40.0 * i / 100 for i, t in enumerate(grid)):
        raise RuntimeError("grid does not equal the frozen 101-point rule")
    if grid_doc.get("binary64_be_hex") != [struct.pack(">d", t).hex() for t in grid]:
        raise RuntimeError("grid binary64 identity does not match its decimal values")
    if grid_doc.get("sha256_json_times") != canonical_times_hash:
        raise RuntimeError("grid's canonical time hash does not match its values")
    if reference.get("status") != "ok" or reference.get("grid_sha256") != canonical_times_hash:
        raise RuntimeError("reference is failed or bound to another grid")
    if manifest.get("grid_file_sha256") != sha(pathlib.Path(args.grid).read_bytes()):
        raise RuntimeError("manifest is bound to a different grid file")
    if manifest.get("reference_file_sha256") != sha(pathlib.Path(args.reference).read_bytes()):
        raise RuntimeError("manifest is bound to a different reference file")
    ladder = reference.get("radau_ladder", [])
    lsoda = reference.get("lsoda_check", {})
    if len(ladder) != 3 or not all(row.get("success") and row.get("times") == grid and len(row.get("states", [])) == 101 for row in ladder):
        raise RuntimeError("reference lacks the three complete Radau ladder receipts")
    validate_reference_center(reference, grid)
    if not lsoda.get("success") or lsoda.get("times") != grid or len(lsoda.get("states", [])) != 101:
        raise RuntimeError("reference lacks a complete LSODA cross-check")
    if (
        reference.get("solver_rtol") != 1.0e-8
        or reference.get("solver_atol") != [1.0e-11] * 3
        or reference.get("initial_state") != [1.0, 0.0, 0.0]
        or reference.get("span") != [0.0, 40.0]
        or reference.get("reference_stability_threshold") != THRESHOLD
        or [row.get("rtol") for row in ladder] != [1.0e-10, 1.0e-12, 1.0e-13]
        or lsoda.get("rtol") != 1.0e-12
    ):
        raise RuntimeError("reference is bound to different comparison tolerances")
    computed_spreads = {
        "fine_vs_previous_radau_max_grid_wrms": max(normalized_wrms(a, b, 1.0e-11, 1.0e-8) for a, b in zip(ladder[-1]["states"], ladder[-2]["states"])),
        "fine_vs_lsoda_max_grid_wrms": max(normalized_wrms(a, b, 1.0e-11, 1.0e-8) for a, b in zip(ladder[-1]["states"], lsoda["states"])),
    }
    for name, computed in computed_spreads.items():
        if abs(computed - reference["reference_spreads_empirical_not_certified"][name]) > max(1.0e-12, abs(computed) * 1.0e-10):
            raise RuntimeError(f"recorded reference spread differs from recomputed value: {name}")
    all_rows = load_jsonl(args.samples)
    for row in all_rows:
        binary_kind = "allocator" if row.get("protocol") == "separate_allocator_run" else "timing"
        expected_binary = manifest.get("binary_sha256", {}).get(binary_kind, {}).get("sha256")
        if row.get("binary_sha256") != expected_binary:
            row["status"] = "binary_identity_mismatch"
            row["error"] = f"runtime binary does not match manifest {binary_kind} binary"
        if row.get("grid_sha256") not in (None, sha(pathlib.Path(args.grid).read_bytes())):
            row["status"] = "identity_mismatch"
            row["error"] = "grid file identity mismatch"
        if row.get("manifest_sha256") not in (None, sha(pathlib.Path(args.manifest).read_bytes())):
            row["status"] = "identity_mismatch"
            row["error"] = "manifest identity mismatch"
        if row.get("status") == "ok":
            if row.get("times") != grid:
                row["status"] = "output_grid_mismatch"
                row["error"] = "returned time grid differs from frozen 101-point grid"
            elif len(row.get("states", [])) != len(grid) or any(
                len(y) != 3 or not all(isinstance(v, (int, float)) and math.isfinite(v) for v in y)
                for y in row["states"]
            ):
                row["status"] = "invalid_output"
                row["error"] = "state output has wrong shape or non-finite values"
    result = {
        "schema": "vigilode-a07-validated-report-v1", "source_manifest_sha256": sha(pathlib.Path(args.manifest).read_bytes()),
        "grid_sha256": sha(pathlib.Path(args.grid).read_bytes()), "reference_sha256": sha(pathlib.Path(args.reference).read_bytes()),
        "contract": {"problem": "Robertson", "initial_state": [1, 0, 0], "span": [0, 40], "output_points": 101,
                     "rtol": 1.0e-8, "atol": [1.0e-11] * 3, "metric": "max over grid of component WRMS normalized by atol+rtol*abs(reference)",
                     "reference_uncertainty": "empirical spread; not a rigorous enclosure", "reference_threshold": THRESHOLD},
        "environment": {"head": manifest.get("head"), "branch": manifest.get("branch"), "rustc_vv": manifest.get("rustc_vv"), "cargo_version": manifest.get("cargo_version"),
                        "timing_authority": "diagnostic_only"},
        "row_count": len(all_rows), "rows": all_rows, "campaign_validation": validate_campaign_rows(all_rows),
        "accuracy": errors(all_rows, reference, grid),
        "allocator_results": [r for r in all_rows if r.get("protocol") == "separate_allocator_run"],
        "timing": [timing_summary(all_rows, "single_cold_process"), timing_summary(all_rows, "warm_abba")],
    }
    result["accuracy"]["accuracy_overlap_gate"] = bool(result["accuracy"]["accuracy_overlap_gate"] and result["campaign_validation"]["valid"])
    result["speed_measurement_accuracy_eligible"] = result["accuracy"]["accuracy_overlap_gate"]
    result["campaign_valid"] = result["campaign_validation"]["valid"]
    result["speedup_claim"] = "NOT_ASSERTED; wall-clock timing authority is diagnostic_only"
    out = pathlib.Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")
    md = ["# A07 Robertson comparison", "", f"- Scientific accuracy overlap gate: **{result['accuracy']['accuracy_overlap_gate']}** (empirical reference stability only).",
          f"- Speed measurement accuracy gate: **{result['speed_measurement_accuracy_eligible']}**; no speedup claim is made and timing authority remains `diagnostic_only`.",
          f"- Rows retained: {len(all_rows)}; source HEAD `{manifest.get('head')}`.",
          f"- Timing binary SHA-256: `{manifest.get('binary_sha256', {}).get('timing', {}).get('sha256')}`.",
          f"- Allocator binary SHA-256: `{manifest.get('binary_sha256', {}).get('allocator', {}).get('sha256')}`.", "", "## Accuracy", "",
          f"- Reference spreads (max-grid normalized WRMS): `{json.dumps(result['accuracy']['reference_spreads'], sort_keys=True)}`."]
    for arm, data in result["accuracy"]["arms"].items():
        md.append(f"- {arm}: {data['successful_complete_rows']}/{data['rows']} complete successful; max distance={data['max_grid_wrms_distance_to_fine_radau']}; interval={data['empirical_interval']}; gate={data['accuracy_gate']}.")
    md.extend(["", "## Timing diagnostics", ""])
    for timing in result["timing"]:
        md.append(f"- {timing['protocol']}: paired units={timing['paired_units']}, Diffsol/native median ratio={timing['diffsol_over_native_median_ratio']}, bootstrap interval={timing['bootstrap_95pct_interval']}.")
    md.extend(["", "The report is diagnostic. The empirical reference spread is not a rigorous enclosure; no production or universal speed claim follows.", ""])
    md.extend(["", "## Allocation audit", ""])
    for row in result["allocator_results"]:
        md.append(f"- {row.get('arm')}: status={row.get('status')}, allocations={row.get('allocations')}, allocated_bytes={row.get('allocated_bytes')}.")
    md.extend(["", f"Campaign schema/count/order validation: **{result['campaign_valid']}** ({json.dumps(result['campaign_validation']['counts'], sort_keys=True)}).", ""])
    pathlib.Path(args.markdown).write_text("\n".join(md), encoding="utf-8")
    return 0 if result["accuracy"]["accuracy_overlap_gate"] and result["campaign_valid"] else 4


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    cap = sub.add_parser("capture")
    cap.add_argument("--grid", required=True)
    cap.add_argument("--reference", required=True)
    cap.add_argument("--timing-binary", required=True)
    cap.add_argument("--allocator-binary", required=True)
    cap.add_argument("--out", required=True)
    val = sub.add_parser("validate")
    val.add_argument("--grid", required=True)
    val.add_argument("--reference", required=True)
    val.add_argument("--manifest", required=True)
    val.add_argument("--samples", nargs="+", required=True)
    val.add_argument("--out", required=True)
    val.add_argument("--markdown", required=True)
    args = parser.parse_args()
    if args.command == "capture":
        capture_manifest(args)
        return 0
    return validate(args)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:
        print(f"A07 validation failed: {exc}", file=sys.stderr)
        sys.exit(2)
