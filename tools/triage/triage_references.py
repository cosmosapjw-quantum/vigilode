#!/usr/bin/env python3
"""Reference states of the component-triage corpus: REFERENCES.json, one entry per (problem, grid).

* `exact` problems: `rodas5p triage-exact --problem P --grid G` evaluates the closed-form solution (not the
  solver); stdout {"schema": "vigilode-triage-exact-v1", "problem", "grid", "dimension", "output_times",
  "output_times_hex", "states_hex"}; the times are compared bit for bit through `output_times_hex`. rho = 0 (rounding of the f64 closed-form evaluation is not bounded; recorded in `rho_rule`).
  Without `--binary` the entry is null with reason `exact_requires_binary`.
* `two_method` problems: SciPy solve_ivp with the methods of CORPUS `reference.methods` (`scipy-radau`,
  `scipy-lsoda`, `scipy-bdf`, or the bare SciPy names; default Radau, then LSODA; the first is canonical) at rtol = REFERENCE_RTOL of tools/stiff_benchmark_scipy.py (1e-13) and
  atol = atol_scale_P * that rtol, on the output grid, restarted at breakpoints. The NumPy right-hand sides and
  Jacobians are imported from tools/stiff_benchmark_scipy.py and tools/reference_v2/generate_references_v2.py
  (tools/triage/triage_models.py). A problem without a NumPy model gets a null entry with reason
  `no_numpy_model`; a failed method gives a null entry with reason `method_failed`.

    rho[t][i] = 2 * max over non-canonical methods m of |y_canonical[t][i] - y_m[t][i]|

States and rho are 16-digit binary64 hex. The Rust solver is never used for a reference (no self-reference).
Output times are the contract grid (tools/triage/triage_common.py::grid_times) unless `--times-from RUNS.jsonl`
takes the exact times the binary printed (all rows of a (problem, grid) must agree bit for bit, have exact
endpoints and lie within 1e-12 * span of the formula). Python, NumPy and SciPy versions are recorded; wall time
is not (the output is byte-deterministic for fixed inputs and library versions).
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402
import triage_models as tm  # noqa: E402

DEFAULT_METHODS = ("Radau", "LSODA")
METHOD_NAMES = {"scipy-radau": "Radau", "scipy-lsoda": "LSODA", "scipy-bdf": "BDF", "Radau": "Radau",
                "LSODA": "LSODA", "BDF": "BDF"}
GRID_CLOSENESS = 1.0e-12  # relative to the span: guards --times-from against a different grid, not a threshold
RHO_RULE_TWO = "2 x max over non-canonical methods of |canonical - method| per output time and component"
RHO_RULE_EXACT = "exact closed form: rho = 0 (f64 evaluation rounding not bounded)"


def times_from_runs(paths: list[Path]) -> dict[tuple[str, str], list[float]]:
    found: dict[tuple[str, str], list[float]] = {}
    for path in paths:
        for row in tc.load_jsonl(path):
            times = row.get("output_times")
            if not isinstance(times, list) or not times or row.get("success") is not True:
                continue
            key = (row.get("problem"), row.get("grid"))
            if key in found and found[key] != times:
                raise tc.TriageError(f"rows of {key} disagree on output_times")
            found[key] = times
    return found


def check_times(times: list[float], formula: list[float]) -> str | None:
    if len(times) != len(formula):
        return f"{len(times)} times != {len(formula)} of the grid"
    if times[0] != formula[0] or times[-1] != formula[-1]:
        return "endpoints differ from t_span"
    span = abs(formula[-1] - formula[0]) or 1.0
    worst = max(abs(a - b) for a, b in zip(times, formula))
    if worst > GRID_CLOSENESS * span:
        return f"times differ from the grid formula by {worst!r}"
    return None


def null_entry(pid: str, grid: str, kind, reason: str, times: list[float], extra: dict | None = None) -> dict:
    entry = {"problem": pid, "grid": grid, "kind": kind, "status": "null", "reason": reason,
             "output_times": times, "states_hex": None, "rho_hex": None, "rho_rule": None, "methods": [],
             "canonical_method": None, "model_source": None}
    entry.update(extra or {})
    return entry


def exact_entry(binary: str | None, problem: dict, grid: str, times: list[float]) -> dict:
    pid = problem["id"]
    if binary is None:
        return null_entry(pid, grid, "exact", "exact_requires_binary", times)
    proc = subprocess.run([binary, "triage-exact", "--problem", pid, "--grid", grid], capture_output=True, text=True)
    if proc.returncode != 0:
        return null_entry(pid, grid, "exact", f"triage-exact: {tc.describe_exit(proc.returncode)}: "
                                              f"{tc.tail(proc.stderr, 400)}", times)
    doc = tc.parse_json_output(proc.stdout)
    if doc is None or doc.get("schema") != tc.EXACT_SCHEMA or doc.get("problem") != pid or doc.get("grid") != grid:
        return null_entry(pid, grid, "exact", "triage-exact printed no matching vigilode-triage-exact-v1 document",
                          times)
    if doc.get("dimension") is not None and doc["dimension"] != problem.get("dimension"):
        return null_entry(pid, grid, "exact", f"triage-exact dimension {doc['dimension']} != CORPUS "
                                              f"{problem.get('dimension')}", times)
    try:
        exact_times = ([tc.unhex64(h) for h in doc["output_times_hex"]] if doc.get("output_times_hex") is not None
                       else [float(t) for t in doc.get("output_times") or []])
    except (tc.TriageError, TypeError, ValueError) as exc:
        return null_entry(pid, grid, "exact", f"triage-exact output times: {exc}", times)
    if [tc.hex64(t) for t in exact_times] != [tc.hex64(t) for t in times]:
        return null_entry(pid, grid, "exact", "triage-exact output times differ from the reference grid (bitwise)",
                          times)
    try:
        states = tc.hex_matrix(doc.get("states_hex"))
    except tc.TriageError as exc:
        return null_entry(pid, grid, "exact", f"triage-exact states: {exc}", times)
    dim = problem.get("dimension")
    if len(states) != len(times) or any(len(s) != dim for s in states):
        return null_entry(pid, grid, "exact", f"triage-exact states shape != {len(times)} x {dim}", times)
    if not all(tc.finite(v) for s in states for v in s):
        return null_entry(pid, grid, "exact", "triage-exact states are not finite", times)
    zero = tc.hex64(0.0)
    return {"problem": pid, "grid": grid, "kind": "exact", "status": "ok", "reason": None, "output_times": times,
            "states_hex": [[tc.hex64(v) for v in s] for s in states], "rho_hex": [[zero] * dim for _ in times],
            "rho_rule": RHO_RULE_EXACT, "methods": [], "canonical_method": None,
            "model_source": "rodas5p triage-exact (closed form)"}


def two_method_entry(problem: dict, grid: str, times: list[float]) -> dict:
    import numpy as np

    pid = problem["id"]
    model = tm.model_for(problem)
    if model is None:
        return null_entry(pid, grid, "two_method", "no_numpy_model", times)
    if model.dimension != problem.get("dimension"):
        return null_entry(pid, grid, "two_method", f"NumPy model dimension {model.dimension} != CORPUS "
                                                   f"{problem.get('dimension')}", times)
    requested = list((problem.get("reference") or {}).get("methods") or DEFAULT_METHODS)
    unknown = [m for m in requested if m not in METHOD_NAMES]
    if unknown:
        return null_entry(pid, grid, "two_method", f"unknown reference method(s) {unknown}", times)
    methods = [METHOD_NAMES[m] for m in requested]
    rtol = float(tm.scipy_tool().REFERENCE_RTOL)
    atol = float(problem["atol_scale"]) * rtol
    results, infos = [], []
    for method in methods:
        states, info = tm.solve_on_grid(model, problem["t_span"], times, method, rtol, atol)
        infos.append(info)
        results.append(states)
    if any(s is None for s in results) or len(results) < 2:
        reason = "method_failed" if any(s is None for s in results) else "two_method needs at least two methods"
        return null_entry(pid, grid, "two_method", reason, times,
                          {"methods": infos, "canonical_method": methods[0], "model_source": model.source})
    canonical = results[0]
    if not np.isfinite(canonical).all() or not all(np.isfinite(r).all() for r in results):
        return null_entry(pid, grid, "two_method", "non-finite reference state", times,
                          {"methods": infos, "canonical_method": methods[0], "model_source": model.source})
    rho = 2.0 * np.max(np.stack([np.abs(canonical - r) for r in results[1:]]), axis=0)
    return {"problem": pid, "grid": grid, "kind": "two_method", "status": "ok", "reason": None,
            "output_times": times, "states_hex": [[tc.hex64(v) for v in row] for row in canonical],
            "rho_hex": [[tc.hex64(v) for v in row] for row in rho], "rho_rule": RHO_RULE_TWO, "methods": infos,
            "canonical_method": methods[0], "model_source": model.source}


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path, help="REFERENCES.json")
    parser.add_argument("--binary", help="rodas5p binary for `triage-exact` (exact problems only)")
    parser.add_argument("--times-from", type=Path, action="append", default=[],
                        help="RUNS.jsonl whose output_times define the grid (repeatable)")
    parser.add_argument("--problem", action="append", help="only this problem (repeatable)")
    parser.add_argument("--grid", action="append", choices=tc.GRIDS, help="only this grid (repeatable)")
    parser.add_argument("--force", action="store_true", help="overwrite an existing output")
    args = parser.parse_args(argv)
    tc.refuse_existing(args.output, args.force)
    try:
        tm.require_numpy()
        corpus = tc.load_json(args.corpus)
        run_times = times_from_runs(args.times_from)
    except (tc.TriageError, tm.ModelsUnavailable) as exc:
        print("error:", exc, file=sys.stderr)
        return 2
    grids = [g for g in (corpus.get("sweep") or {}).get("grids", []) if args.grid is None or g in args.grid]
    entries = []
    for pid, problem in sorted(tc.corpus_problems(corpus).items()):
        if args.problem is not None and pid not in args.problem:
            continue
        for grid in grids:
            times = tc.grid_times(problem["t_span"], grid)
            source_times = run_times.get((pid, grid))
            if source_times is not None:
                problem_with_times = check_times(source_times, times)
                if problem_with_times:
                    entries.append(null_entry(pid, grid, (problem.get("reference") or {}).get("kind"),
                                              f"--times-from grid rejected: {problem_with_times}", times))
                    continue
                times = source_times
            kind = (problem.get("reference") or {}).get("kind")
            if kind == "exact":
                entry = exact_entry(args.binary, problem, grid, times)
            elif kind == "two_method":
                entry = two_method_entry(problem, grid, times)
            else:
                entry = null_entry(pid, grid, kind, f"unknown reference kind {kind!r}", times)
            print(f"{pid} {grid}: {entry['status']} {entry['reason'] or ''}", file=sys.stderr, flush=True)
            entries.append(entry)
    doc = {"schema": tc.REFERENCES_SCHEMA, "generator": "tools/triage/triage_references.py",
           "runtime": tm.versions(), "corpus_sha256": tc.sha256_file(args.corpus),
           "times_source": "runs" if args.times_from else "grid_formula",
           "reference_rtol": float(tm.scipy_tool().REFERENCE_RTOL),
           "references": entries}
    tc.write_text(args.output, tc.dumps(doc) + "\n", args.force)
    print(f"wrote {args.output} ({len(entries)} entries)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
