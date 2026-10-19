#!/usr/bin/env python3
"""External comparator arms of the component triage, as rows of the contract row schema (vigilode-triage-row-v1).

Arms and sources:

* `ext.scipy-radau`, `ext.scipy-bdf`, `ext.scipy-lsoda`: SciPy solve_ivp (Radau / BDF / LSODA) with t_eval on
  the cell's output grid, rtol = cell rtol, atol = rtol * atol_scale_P, first_step = h0 = h0_scale * h0_base,
  max_step = tf - t0, restarted at breakpoints, NumPy model of tools/triage/triage_models.py (imported from
  tools/stiff_benchmark_scipy.py and tools/reference_v2/generate_references_v2.py). Raw SciPy counters go to
  `extra_work` = {nfev, njev, nlu, upstream}; `counters` is null (no WorkCounters exist for SciPy).
  `attempts`/`accepted_steps`/`rejected_steps` are null (solve_ivp does not report them).
* `ext.cvode`, `ext.hairer-radau5`, `ext.hairer-rodas`: tools/native_stiff (built by tools/native_stiff/build.sh)
  `native_stiff run <problem> <cvode-bdf|hairer-radau5|hairer-rodas> <rtol> <reps> 0`. The driver reports the
  final state only, uses its own default initial step and its own problem table, so: grid `uniform101` and
  h0_scale != 1 cells are `unavailable` (reasons recorded); the h0_scale = 1 endpoint cell is run and its row has
  h0 = null with extra_work.h0_policy = "solver_default"; a driver atol that differs from the cell atol makes
  the row `unavailable` (upstream output kept). Without the driver every native cell is `unavailable`.
* `ext.diffsol-bdf`: adapter for the JSONL rows of tools/benchmarks/diffsol/a07 (`--diffsol-rows`, Robertson,
  rtol/atol read from the a07 source constants, 101-point grid). The first `single_cold_process` row of arm
  `diffsol` is the cell's row; callback counters of a `separate_allocator_run` row give extra_work.nfev.
  Without `--diffsol-rows`, or for any other cell, rows are `unavailable` with the reason.

External rows are admissible per single row (THRESHOLDS.external: the h0 rule applies to internal arms only),
so the plan has one cell per (arm, problem, rtol, grid): the nominal h0_scale = 1. Every planned cell gets a
row; none is skipped silently. Python wall time is never recorded (it is not a cost).
EXTERNAL.jsonl is resumable like RUNS.jsonl and written in plan order; `--force` truncates it.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402
import triage_models as tm  # noqa: E402

SCIPY_ARMS = {"ext.scipy-radau": "Radau", "ext.scipy-bdf": "BDF", "ext.scipy-lsoda": "LSODA"}
NATIVE_ARMS = {"ext.cvode": "cvode-bdf", "ext.hairer-radau5": "hairer-radau5", "ext.hairer-rodas": "hairer-rodas"}
DIFFSOL_ARM = "ext.diffsol-bdf"
EXTERNAL_ARMS = tuple(SCIPY_ARMS) + tuple(NATIVE_ARMS) + (DIFFSOL_ARM,)
A07_SOURCE = tc.TOOLS / "benchmarks" / "diffsol" / "a07" / "src" / "main.rs"
COUNTER_SEMANTICS = "external-upstream-raw"
NOMINAL_H0 = 1.0


def same_tolerance(a: float, b: float) -> bool:
    """Equal up to 4 ulps: a literal 1e-11 upstream and rtol * atol_scale = 1e-8 * 1e-3 here are the same cell.
    Representation slack only, not a triage threshold."""
    return abs(float(a) - float(b)) <= 4.0 * sys.float_info.epsilon * max(abs(float(a)), abs(float(b)))


def base_row(cell: dict, problem: dict, campaign: str, prov: dict, binary_sha: str | None,
             repetitions: int) -> dict:
    h0 = cell["h0_scale"] * tc.h0_base(problem)
    return {
        "schema": tc.ROW_SCHEMA, "campaign": campaign, "arm": cell["arm"], "components": [],
        "problem": cell["problem"], "tier": problem.get("tier"), "dimension": problem.get("dimension"),
        "rtol": cell["rtol"], "atol": cell["rtol"] * float(problem["atol_scale"]), "grid": cell["grid"],
        "h0_scale": cell["h0_scale"], "h0": h0, "status": None, "error": None, "success": False,
        "attempts": None, "accepted_steps": None, "rejected_steps": None, "output_times": [], "states_hex": [],
        "counters": None, "extra_work": None, "counter_semantics": COUNTER_SEMANTICS, "deterministic": None,
        "repetitions": repetitions, "binary": {"sha256": binary_sha, "profile": "external"},
        "source": {"commit": prov.get("commit"), "tree_clean": prov.get("tree_clean")},
    }


def unavailable(row: dict, reason: str, extra: dict | None = None) -> dict:
    row.update(status="unavailable", error=reason, success=False, extra_work=extra)
    return row


# ------------------------------------------------------------------------------------------- SciPy
def scipy_cell(cell: dict, problem: dict, campaign: str, prov: dict, repetitions: int) -> dict:
    row = base_row(cell, problem, campaign, prov, None, repetitions)
    try:
        model = tm.model_for(problem)
    except tm.ModelsUnavailable as exc:
        return unavailable(row, str(exc))
    if model is None:
        return unavailable(row, "no_numpy_model")
    if model.dimension != problem.get("dimension"):
        return unavailable(row, f"NumPy model dimension {model.dimension} != CORPUS {problem.get('dimension')}")
    import numpy as np

    times = tc.grid_times(problem["t_span"], cell["grid"])
    t0, tf = problem["t_span"]
    finals, info = [], None
    for _ in range(repetitions):
        states, info = tm.solve_on_grid(model, problem["t_span"], times, SCIPY_ARMS[cell["arm"]], row["rtol"],
                                        row["atol"], first_step=row["h0"], max_step=float(tf) - float(t0))
        finals.append(states)
    first = finals[0]
    row["extra_work"] = {"nfev": info["nfev"], "njev": info["njev"], "nlu": info["nlu"],
                         "upstream": "scipy.integrate.solve_ivp", "method": SCIPY_ARMS[cell["arm"]],
                         "segments": info["segments"], "h0_policy": "first_step = h0"}
    if first is None:
        row.update(status="solver_failure", error=info["message"], success=False)
        row["deterministic"] = all(f is None for f in finals)
        return row
    row["deterministic"] = all(f is not None and np.array_equal(f, first) for f in finals)
    row.update(status="ok", success=True, output_times=times,
               states_hex=[[tc.hex64(v) for v in s] for s in first])
    return row


# ------------------------------------------------------------------------------------------ native
def native_cell(cell: dict, problem: dict, campaign: str, prov: dict, driver: str | None, driver_sha: str | None,
                repetitions: int) -> dict:
    row = base_row(cell, problem, campaign, prov, driver_sha, repetitions)
    if driver is None:
        return unavailable(row, "native driver not given or not found (build with tools/native_stiff/build.sh)")
    if cell["grid"] != "endpoint":
        return unavailable(row, "native driver reports the final state only (grid endpoint)")
    if cell["h0_scale"] != 1.0:
        return unavailable(row, "native driver uses the solver's default initial step; h0 is not controllable")
    proc = subprocess.run([driver, "run", cell["problem"], NATIVE_ARMS[cell["arm"]], tc.fmt_float(cell["rtol"]),
                           str(repetitions), "0"], capture_output=True, text=True)
    if proc.returncode != 0:
        reason = f"native driver: {tc.describe_exit(proc.returncode)}: {tc.tail(proc.stderr, 400).strip()}"
        if proc.returncode == 2:
            return unavailable(row, reason)
        row.update(status="process_abort", error=reason, success=False)
        row["process"] = {"kind": "abort", "exit_status": proc.returncode, "signal": None,
                          "stderr_tail": tc.tail(proc.stderr), "stdout_tail": tc.tail(proc.stdout)}
        return row
    lines = [line for line in proc.stdout.splitlines() if line.startswith("{")]
    upstream = tc.parse_json_output(lines[-1]) if lines else None
    if upstream is None:
        row.update(status="error", error="native driver printed no JSON row", success=False)
        return row
    upstream.pop("wall_seconds", None)  # wall time is never a cost here
    upstream.pop("wall_median", None)
    counters = upstream.get("counters") or {}
    extra = {"nfev": counters.get("rhs_evaluations"), "njev": counters.get("jacobian_builds"),
             "nlu": counters.get("direct_factorizations"), "upstream": "tools/native_stiff",
             "solver": NATIVE_ARMS[cell["arm"]], "h0_policy": "solver_default", "t_span_verified": False,
             "raw": upstream}
    if not tc.finite(upstream.get("atol")) or not same_tolerance(upstream["atol"], row["atol"]):
        return unavailable(row, f"native atol {upstream.get('atol')!r} != cell atol {row['atol']!r}", extra)
    row.update(h0=None, extra_work=extra, accepted_steps=upstream.get("accepted_steps"),
               rejected_steps=upstream.get("rejected_steps"), deterministic=upstream.get("deterministic"))
    if upstream.get("status") != "completed":
        row.update(status="solver_failure", error=upstream.get("message") or "failed", success=False)
        return row
    final = upstream.get("final_state")
    if not isinstance(final, list) or len(final) != problem.get("dimension"):
        row.update(status="error", error="native final_state has the wrong length", success=False)
        return row
    t0, tf = float(problem["t_span"][0]), float(problem["t_span"][1])
    y0_hex = None
    try:
        model = tm.model_for(problem)
        y0_hex = [tc.hex64(v) for v in model.y0] if model is not None else None
    except tm.ModelsUnavailable:
        y0_hex = None
    if y0_hex is None:
        row.update(status="error", error="native driver reports no initial state and no NumPy model gives it",
                   success=False)
        return row
    row.update(status="ok", success=True, output_times=[t0, tf],
               states_hex=[y0_hex, [tc.hex64(float(v)) for v in final]])
    return row


# ----------------------------------------------------------------------------------------- diffsol
def a07_constants(source: Path = A07_SOURCE) -> dict:
    text = source.read_text()
    out = {}
    for name in ("RTOL", "ATOL", "POINTS"):
        match = re.search(rf"const {name}: \w+ = ([0-9eE.+_-]+);", text)
        if match is None:
            raise tc.TriageError(f"{source}: constant {name} not found")
        out[name] = float(match.group(1).replace("_", ""))
    return out


def diffsol_rows(path: Path | None) -> tuple[list[dict], dict | None]:
    if path is None:
        return [], None
    rows = [r for r in tc.load_jsonl(path) if r.get("schema") == "vigilode-a07-run-v1" and r.get("arm") == "diffsol"]
    return rows, a07_constants()


def diffsol_cell(cell: dict, problem: dict, campaign: str, prov: dict, rows: list[dict], const: dict | None,
                 problem_id: str) -> dict:
    row = base_row(cell, problem, campaign, prov, None, 1)
    if const is None:
        return unavailable(row, "no a07 diffsol rows given (--diffsol-rows)")
    if (cell["problem"] != problem_id or cell["rtol"] != const["RTOL"] or cell["grid"] != "uniform101"
            or cell["h0_scale"] != 1.0):
        return unavailable(row, f"a07 covers {problem_id} rtol {const['RTOL']!r} uniform101 with the solver's "
                                f"default initial step only")
    cold = [r for r in rows if r.get("protocol") == "single_cold_process"]
    if not cold:
        return unavailable(row, "a07 rows contain no single_cold_process diffsol row")
    primary = cold[0]
    alloc = next((r for r in rows if r.get("protocol") == "separate_allocator_run"), None)
    callbacks = (alloc or {}).get("callback_counters")
    ok_states = [r.get("states") for r in rows if r.get("status") == "ok"]
    extra = {"nfev": (callbacks or {}).get("rhs_evaluations"), "njev": (callbacks or {}).get("jacobian_builds"),
             "nlu": None, "upstream": "tools/benchmarks/diffsol/a07", "solver_counters": primary.get("solver_counters"),
             "callback_counters": callbacks, "h0_policy": "solver_default", "a07_rows": len(rows),
             "a07_binary_sha256": primary.get("binary_sha256")}
    if not same_tolerance(const["ATOL"], row["atol"]):
        return unavailable(row, f"a07 atol {const['ATOL']!r} != cell atol {row['atol']!r}", extra)
    row["atol"] = const["ATOL"]  # the tolerance the upstream solver actually used
    row.update(h0=None, extra_work=extra, deterministic=all(s == ok_states[0] for s in ok_states) if ok_states else None,
               binary={"sha256": primary.get("binary_sha256"), "profile": "external"})
    if primary.get("status") != "ok":
        row.update(status="error", error=primary.get("error") or "a07 diffsol row failed", success=False)
        return row
    times, states = primary.get("times"), primary.get("states")
    if not isinstance(times, list) or len(times) != int(const["POINTS"]) or not isinstance(states, list) \
            or len(states) != len(times):
        row.update(status="error", error="a07 row has the wrong number of output times", success=False)
        return row
    row.update(status="ok", success=True, output_times=[float(t) for t in times],
               states_hex=[[tc.hex64(float(v)) for v in s] for s in states])
    return row


# --------------------------------------------------------------------------------------------- main
def plan(corpus: dict, list_doc: dict | None, arms, sel: dict) -> list[dict]:
    sweep = corpus.get("sweep") or {}
    keep = lambda values, chosen: [v for v in values if chosen is None or v in chosen]  # noqa: E731
    rtols = keep([float(r) for r in sweep.get("rtols", [])], sel["rtols"])
    grids = keep(list(sweep.get("grids", [])), sel["grids"])
    h0s = keep([NOMINAL_H0], sel["h0_scales"])  # external rows: one nominal-h0 row per cell
    cells = []
    for pid, p in sorted(tc.corpus_problems(corpus).items()):
        if (sel["problems"] is not None and pid not in sel["problems"]) or \
                (sel["tiers"] is not None and p.get("tier") not in sel["tiers"]):
            continue
        for arm in sorted(arms):
            for rtol in rtols:
                for grid in grids:
                    for h0 in h0s:
                        cells.append({"arm": arm, "problem": pid, "rtol": rtol, "grid": grid, "h0_scale": h0})
    return cells


def run_one(cell, problem, args_dict, prov, driver_sha, diff_rows, diff_const):
    arm = cell["arm"]
    if arm in SCIPY_ARMS:
        return scipy_cell(cell, problem, args_dict["campaign"], prov, args_dict["repetitions"])
    if arm in NATIVE_ARMS:
        return native_cell(cell, problem, args_dict["campaign"], prov, args_dict["native_driver"], driver_sha,
                           args_dict["repetitions"])
    return diffsol_cell(cell, problem, args_dict["campaign"], prov, diff_rows, diff_const,
                        args_dict["diffsol_problem"])


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--list", type=Path, help="triage-list JSON (adds jvp/breakpoints/h0_base of problems)")
    parser.add_argument("--output", required=True, type=Path, help="EXTERNAL.jsonl (appended; resumable)")
    parser.add_argument("--campaign", required=True)
    parser.add_argument("--external-arm", action="append", choices=EXTERNAL_ARMS,
                        help="external arm to run (repeatable; default: all)")
    parser.add_argument("--native-driver", help="tools/native_stiff build output `native_stiff`")
    parser.add_argument("--diffsol-rows", type=Path, help="a07 JSONL rows (vigilode-a07-run-v1)")
    parser.add_argument("--diffsol-problem", default="robertson", help="CORPUS id of the a07 problem")
    parser.add_argument("--repetitions", type=int, default=1, help="runs per cell for the determinism flag")
    parser.add_argument("-j", "--jobs", type=int, default=1, help="parallel worker processes")
    parser.add_argument("--force", action="store_true")
    tc.add_selection_arguments(parser)
    args = parser.parse_args(argv)
    try:
        corpus = tc.load_json(args.corpus)
        list_doc = tc.load_json(args.list) if args.list else None
        diff_rows, diff_const = diffsol_rows(args.diffsol_rows)
    except tc.TriageError as exc:
        for reason in exc.reasons:
            print("error:", reason, file=sys.stderr)
        return 2
    list_problems = {p["id"]: p for p in (list_doc or {}).get("problems", [])}
    problems = {pid: tc.merged_problem(p, list_problems.get(pid)) for pid, p in tc.corpus_problems(corpus).items()}
    sel = tc.selection(args)
    sel = {k: (None if v is None else set(v)) for k, v in sel.items()}
    cells = plan(corpus, list_doc, args.external_arm or EXTERNAL_ARMS, sel)
    driver = args.native_driver if args.native_driver and Path(args.native_driver).is_file() else None
    driver_sha = tc.sha256_file(Path(driver)) if driver else None
    if args.force and args.output.exists():
        args.output.unlink()
    done = {tc.cell_key(r) for r in tc.load_jsonl(args.output)} if args.output.exists() else set()
    todo = [c for c in cells if (args.campaign, c["arm"], c["problem"], c["rtol"], c["grid"], c["h0_scale"])
            not in done]
    print(f"{len(cells)} planned external cells, {len(todo)} to run", file=sys.stderr)
    prov = tc.git_provenance()
    args_dict = {"campaign": args.campaign, "repetitions": max(1, args.repetitions), "native_driver": driver,
                 "diffsol_problem": args.diffsol_problem}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    appender = tc.OrderedAppender(args.output, len(todo))
    pool_type = concurrent.futures.ProcessPoolExecutor if args.jobs > 1 else concurrent.futures.ThreadPoolExecutor
    try:
        with pool_type(max_workers=max(1, args.jobs)) as pool:
            futures = {pool.submit(run_one, c, problems[c["problem"]], args_dict, prov, driver_sha, diff_rows,
                                   diff_const): i for i, c in enumerate(todo)}
            for future in concurrent.futures.as_completed(futures):
                row = future.result()
                print(f"{row['arm']} {row['problem']} rtol={row['rtol']:g} {row['grid']} h0x{row['h0_scale']:g}: "
                      f"{row['status']} {row['error'] or ''}", file=sys.stderr, flush=True)
                appender.add(futures[future], row)
    finally:
        appender.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
