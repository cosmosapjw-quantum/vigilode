#!/usr/bin/env python3
"""Native comparators of the stiff benchmark: SUNDIALS CVODE and Hairer's RADAU5 and RODAS.

Research node: research/stiff_native_benchmark_20261001 (see its PREREGISTRATION.md).

Inputs: the Rust benchmark JSON of the RODAS5P arm (`rodas5p stiff-benchmark --arms rodas5p
--problems ...`) and the driver built by tools/native_stiff/build.sh.  The script checks
right-hand-side and Jacobian parity of the C problems against the Rust samples, computes the
references with tools/stiff_benchmark_scipy.py, runs the four native arms over the Rust
tolerance ladder with the same warmup and repetition counts, records the LU microbenchmarks,
and writes the native rows and the merged matched-error analysis.  Wall seconds are
diagnostics on a shared host.
"""

from __future__ import annotations

import os

for _var in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_var] = "1"

import argparse
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCHEMA = "vigilode-stiff-native-benchmark-v1"
NATIVE_ARMS = ("cvode-bdf", "cvode-bdf-lapack", "hairer-radau5", "hairer-rodas")


def load_scipy_tool():
    spec = importlib.util.spec_from_file_location("stiff_benchmark_scipy", HERE / "stiff_benchmark_scipy.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def json_line(stdout: str):
    """The driver's JSON object; Fortran codes may print notes before it."""
    lines = [line for line in stdout.splitlines() if line.startswith("{") or line.startswith("[")]
    notes = [line.strip() for line in stdout.splitlines() if line.strip() and line not in lines]
    return json.loads(lines[-1]), notes


def parity(driver, rust, tool):
    report = {}
    for pid, samples in rust["parity_samples"].items():
        stdin = "\n".join(" ".join(repr(float(v)) for v in s["y"]) for s in samples) + "\n"
        out = subprocess.run([driver, "parity", pid], input=stdin, capture_output=True, text=True, check=True)
        native, _ = json_line(out.stdout)
        worst_f = max(tool.rel_diff(n["f"], s["f"]) for n, s in zip(native, samples))
        worst_j = max(tool.rel_diff(n["jacobian"], s["jacobian"]) for n, s in zip(native, samples))
        report[pid] = {"rhs_max_rel_diff": worst_f, "jacobian_max_rel_diff": worst_j,
                       "samples": len(native),
                       "ok": len(native) == len(samples) and max(worst_f, worst_j) <= tool.PARITY_LIMIT}
    return report


def native_rows(driver, rust, tool):
    rows = []
    for problem in rust["problems"]:
        for arm in NATIVE_ARMS:
            for rtol in tool.rust_tolerances(rust):
                out = subprocess.run([driver, "run", problem["id"], arm, repr(rtol),
                                      str(rust["repetitions"]), str(rust["warmups"])],
                                     capture_output=True, text=True, check=True)
                row, notes = json_line(out.stdout)
                if notes:
                    row["driver_notes"] = notes
                print(f"{problem['id']} {arm} rtol={rtol:g}: {row['status']} median "
                      f"{row['wall_median']:.4e} s", file=sys.stderr)
                rows.append(row)
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust", required=True, type=Path)
    parser.add_argument("--driver", required=True)
    parser.add_argument("--native-output", required=True, type=Path)
    parser.add_argument("--analysis-output", required=True, type=Path)
    args = parser.parse_args()
    for path in (args.native_output, args.analysis_output):
        if path.exists():
            raise SystemExit(f"immutable output already exists: {path}")
    tool = load_scipy_tool()
    rust = json.loads(args.rust.read_text())
    build_info = Path(args.driver).with_name("BUILD_INFO.txt")
    report = {
        "schema": SCHEMA,
        "build": build_info.read_text().splitlines() if build_info.exists() else None,
        "timing_status": "DIAGNOSTIC: wall seconds on a shared host; statistical timing authority is on hold",
        "parity": parity(args.driver, rust, tool),
        "references": {p["id"]: tool.reference(p["id"], p) for p in rust["problems"]},
        "lu_microbench": [json_line(subprocess.run([args.driver, "lu-bench", str(cells), "51"],
                                                    capture_output=True, text=True, check=True).stdout)[0]
                          for cells in (50, 200)],
    }
    report["rows"] = native_rows(args.driver, rust, tool)
    args.native_output.write_text(json.dumps(report, indent=1) + "\n")
    result = tool.analysis(rust, report)
    result["validity_gate"]["native_deterministic"] = all(r["deterministic"] for r in report["rows"])
    result["verdict"] = "PASS" if all(result["validity_gate"].values()) else "FAIL"
    result["schema"] = SCHEMA + "-analysis"
    args.analysis_output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({"verdict": result["verdict"], "gate": result["validity_gate"]}))


if __name__ == "__main__":
    main()
