#!/usr/bin/env python3
"""Bounded malformed-evidence probes; never rewrite or rerun a solver study.

Preregistered in this node, section C. Helper probes call only an existing
endpoint metric. End-to-end probes run ALG06's checker on temporary, explicitly
malformed copies of its published inputs. The historical verdict is not
re-evaluated or replaced. No production code or original artifact is changed.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import struct
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
CHECKERS = {
    "ALG04": ROOT / "tools/alg04_coupled_target_v2_check.py",
    "ALG05": ROOT / "tools/alg05_controller_v2_check.py",
    "ALG06": ROOT / "tools/alg06_guard_v2_check.py",
}
BASE = ROOT / "research/alg03_stage_budget_guard_20261008/BASE.json"
RUNS = ROOT / "research/alg06_guard_v2_20261010/RUNS.json"
RESULTS = ROOT / "research/alg06_guard_v2_20261010/RESULTS.json"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def bits(value: float) -> str:
    return struct.pack(">d", value).hex()


def json_value(value: float) -> float | str:
    if math.isnan(value):
        return "NaN"
    if math.isinf(value):
        return "+Infinity" if value > 0 else "-Infinity"
    return value


def load_checker(name: str, path: Path):
    spec = importlib.util.spec_from_file_location("reaudit_probe_" + name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load checker: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def helper_probes() -> list[dict]:
    # NaN is intentionally in the second coordinate: max(0, NaN) can be 0.
    cases = [
        ("valid_control", [0.0, 1.0], [0.0, 1.0]),
        ("empty", [], []),
        ("truncated", [0.0], [0.0, 1.0]),
        ("nan_second", [0.0, float("nan")], [0.0, 0.0]),
        ("infinity_second", [0.0, float("inf")], [0.0, 0.0]),
    ]
    observations = []
    for name, path in CHECKERS.items():
        module = load_checker(name, path)
        for case, state, reference in cases:
            row = {
                "scope": "helper_only",
                "checker": name,
                "case": case,
                "state_bits": [bits(v) for v in state],
                "reference_bits": [bits(v) for v in reference],
                "conclusion_ceiling": "Metric behavior only; no full-checker verdict inferred.",
            }
            try:
                if name == "ALG05":
                    value = module.endpoint_error([bits(v) for v in state], reference)
                else:
                    value = module.componentwise(state, reference)
                row.update(
                    outcome="returned_finite" if math.isfinite(value) else "returned_nonfinite",
                    value=json_value(value),
                )
            except Exception as exc:  # Record the expected empty/shape rejection.
                row.update(outcome="raised", exception_type=type(exc).__name__, message=str(exc))
            expected = (
                "raised" if case == "empty" or (case == "truncated" and name == "ALG05")
                else "returned_nonfinite" if case == "infinity_second"
                else "returned_finite"
            )
            row["expected_from_static_review"] = expected
            row["expectation_matched"] = row["outcome"] == expected
            row["missing_input_rejection_observed"] = (
                case in ("truncated", "nan_second") and row["outcome"] == "returned_finite"
            )
            observations.append(row)
    return observations


def end_to_end_probes(base: dict, runs: dict) -> list[dict]:
    observations = []
    # Fixed cells and mutations; no scan for a favourable checker outcome.
    candidate_key = ("D1", "brusselator-1d-160", 1e-4)
    twin_key = ("D2", "robertson-4e10", 1e-5)

    def key(row: dict) -> tuple:
        return row["group"], row["case"], row["rtol"]

    for case in ("truncated_candidate", "nan_candidate", "wrong_twin", "duplicate_row"):
        mutated = copy.deepcopy(runs)
        selected = next(row for row in mutated["rows"] if key(row) == candidate_key)
        modification: dict = {"case": case, "cell": list(candidate_key)}
        if case == "truncated_candidate":
            # The only retained coordinate exactly equals its reference.
            selected["arms"]["b3"]["y_last"] = base["references"][selected["case"]]["y"][:1]
            modification.update(retained_components=1, declared_dimension=selected["dimension"])
        elif case == "nan_candidate":
            state = list(base["references"][selected["case"]]["y"])
            state[1] = bits(float("nan"))
            selected["arms"]["b3"]["y_last"] = state
            modification.update(nan_component=1, other_components="reference bits")
        elif case == "wrong_twin":
            selected = next(row for row in mutated["rows"] if key(row) == twin_key)
            before = selected["twin"]["y_last"][0]
            selected["twin"]["y_last"][0] = bits(0.0)
            modification.update(cell=list(twin_key), component=0, before=before, after=bits(0.0))
        else:
            # Identical duplicate demonstrates that raw uniqueness is unchecked.
            # Last-wins handling of contradictory duplicates is a static finding,
            # not an additional numerical mutation tested here.
            mutated["rows"].append(copy.deepcopy(selected))
            modification.update(raw_rows_before=len(runs["rows"]), raw_rows_after=len(mutated["rows"]))

        with tempfile.TemporaryDirectory(prefix="vigilode_checker_probe_") as directory:
            tmp = Path(directory)
            base_copy, runs_copy, output = tmp / "BASE.json", tmp / "RUNS.json", tmp / "RESULT.json"
            base_copy.write_bytes(BASE.read_bytes())
            runs_copy.write_text(json.dumps(mutated, allow_nan=False) + "\n")
            # Omit ALG03 reproduction reporting by naming a nonexistent optional
            # input. No historical solver, exporter or scientific campaign runs.
            command = [
                sys.executable, str(CHECKERS["ALG06"]),
                "--base", str(base_copy), "--runs", str(runs_copy),
                "--alg03-runs", str(tmp / "NOT_REQUESTED.json"), "--output", str(output),
            ]
            process = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, timeout=60)
            result = json.loads(output.read_text()) if output.exists() else None
            summary = None
            if result is not None:
                affected_key = twin_key if case == "wrong_twin" else candidate_key
                affected = next(c for c in result["cells"] if key(c) == affected_key)
                summary = {
                    "verdict": result["verdict"],
                    "gate": result["gate"],
                    "canonical_cells": len(result["cells"]),
                    "twins_reproduced": result["reported"]["twins_reproduced"],
                    "affected_cell": list(affected_key),
                    "affected_b3_error": affected["arms"]["b3"]["error"],
                    "affected_twin_error": affected["twin"]["error"],
                    "affected_twin_equals_base": affected["twin"]["equal_to_base"],
                }
            observed = process.returncode == 0 and result is not None and result["verdict"] == "PASS"
            row = {
                "scope": "full_checker_on_malformed_copy",
                "checker": "ALG06",
                "mutation": modification,
                "copied_runs_sha256": digest(runs_copy),
                "returncode": process.returncode,
                "checker_result": summary,
                "stdout": process.stdout,
                "stderr": process.stderr,
                "expected_from_static_review": "PASS despite this malformed evidence",
                "expectation_matched": observed,
                "malformed_copy_passed": observed,
                "conclusion_ceiling": (
                    "Checker validation gap on a synthetic copy; not a defect in the historical run "
                    "or a false acceptance by the ODE solver."
                ),
            }
            if case == "wrong_twin":
                row["expectation_matched"] = observed and summary["twins_reproduced"] is False
            observations.append(row)
    return observations


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path(__file__).with_name("CHECKER_PROBES.json"))
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    # Avoid creating __pycache__ in the protected historical tools directory.
    sys.dont_write_bytecode = True
    sources = [*CHECKERS.values(), BASE, RUNS, RESULTS]
    before = {str(p.relative_to(ROOT)): digest(p) for p in sources}
    base, runs = json.loads(BASE.read_text()), json.loads(RUNS.read_text())
    helpers = helper_probes()
    full = end_to_end_probes(base, runs)
    after = {str(p.relative_to(ROOT)): digest(p) for p in sources}
    unchanged = before == after
    expectations = all(p["expectation_matched"] for p in helpers + full)
    report = {
        "schema": "vigilode-reaudit-checker-malformed-probes-v1",
        "study_status": "COMPLETED" if unchanged and expectations else "REVIEW_REQUIRED",
        "historical_results_policy": "Original inputs/results unchanged; no solver study rerun or verdict replacement.",
        "source_hashes": before,
        "source_hashes_unchanged": unchanged,
        "published_alg06_verdict_read_only": json.loads(RESULTS.read_text())["verdict"],
        "helper_probes": helpers,
        "end_to_end_probes": full,
        "expectations_matched": expectations,
        "checker_authority": "NEEDS_HARDENING" if any(p["malformed_copy_passed"] for p in full) else "UNRESOLVED",
        "no_claims": ["historical data corruption", "false ODE solution acceptance", "production solver regression"],
    }
    args.output.write_text(json.dumps(report, indent=2, allow_nan=False) + "\n")
    print(f"{report['study_status']}: {len(helpers)} helper probes, {len(full)} copied-input checker probes")
    print(f"Historical source hashes unchanged: {unchanged}; checker authority: {report['checker_authority']}")
    return 0 if unchanged and expectations else 1


if __name__ == "__main__":
    raise SystemExit(main())
