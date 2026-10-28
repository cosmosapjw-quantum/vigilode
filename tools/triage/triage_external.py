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
* `ext.diffsol-bdf`, `ext.diffsol-esdirk34`, `ext.diffsol-tr-bdf2`: the isolated Diffsol 0.16.1 harness
  tools/benchmarks/diffsol/triage (`--diffsol-binary`), `diffsol-triage run --problem P --method
  {bdf,esdirk34,tr-bdf2} --rtol R --grid G --campaign ID --repetitions K`, which prints the cell's row itself
  (scalar atol = rtol * atol_scale, OdeSolverMethod::solve_dense on the cell's grid, raw OdeSolverStatistics and
  rhs OpStatistics plus the derived nfev/njev/nlu in extra_work, h0 = null: Diffsol picks its own first step).
  `diffsol-triage list` is read once: a CORPUS problem the harness does not implement, or whose dimension,
  t_span, atol_scale or breakpoints differ from CORPUS, gets an `unavailable` row with the reason, as does every
  cell when the binary is not given. The row is checked against the cell (identity, atol bit for bit, output
  times bit for bit equal to the requested grid); an abort, a timeout (`--diffsol-timeout`), exit 2 or an
  unparseable/mismatched row become `process_abort` rows with `process.kind` as in triage_runner.py.

External rows are admissible per single row (THRESHOLDS.external: the h0 rule applies to internal arms only),
so the plan has one cell per (arm, problem, rtol, grid): the nominal h0_scale = 1. Every planned cell gets a
row; none is skipped silently. Python wall time is never recorded (it is not a cost).
Large rows move their states to `--states-dir` sidecars with `accuracy_inline`, exactly as in
tools/triage/triage_runner.py. EXTERNAL.jsonl is resumable like RUNS.jsonl and written in plan order;
`--force` truncates it.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402
import triage_models as tm  # noqa: E402

SCIPY_ARMS = {"ext.scipy-radau": "Radau", "ext.scipy-bdf": "BDF", "ext.scipy-lsoda": "LSODA"}
NATIVE_ARMS = {"ext.cvode": "cvode-bdf", "ext.hairer-radau5": "hairer-radau5", "ext.hairer-rodas": "hairer-rodas"}
DIFFSOL_ARMS = {"ext.diffsol-bdf": "bdf", "ext.diffsol-esdirk34": "esdirk34", "ext.diffsol-tr-bdf2": "tr-bdf2"}
EXTERNAL_ARMS = tuple(SCIPY_ARMS) + tuple(NATIVE_ARMS) + tuple(DIFFSOL_ARMS)
DIFFSOL_LIST_SCHEMA = "vigilode-diffsol-triage-list-v1"
DIFFSOL_BUILD_HINT = "build tools/benchmarks/diffsol/triage (README there)"
DEFAULT_DIFFSOL_TIMEOUT = 3600.0
COUNTER_SEMANTICS = "external-upstream-raw"
NOMINAL_H0 = 1.0


def same_tolerance(a: float, b: float) -> bool:
    """Equal up to 4 ulps: a literal atol upstream (e.g. 1e-11) and rtol * atol_scale here (1e-8 * 1e-3) are the
    same cell. Representation slack only, not a triage threshold. (Diffsol rows compute rtol * atol_scale
    themselves and are compared exactly.)"""
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
def diffsol_listing(binary: str | None) -> tuple[dict | None, str | None]:
    """`diffsol-triage list` (the harness's problem definitions), or (None, reason)."""
    if binary is None:
        return None, f"Diffsol harness binary not given or not found (--diffsol-binary; {DIFFSOL_BUILD_HINT})"
    try:
        proc = subprocess.run([binary, "list"], capture_output=True, text=True, timeout=120)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return None, f"diffsol-triage list failed: {exc}"
    if proc.returncode != 0:
        return None, f"diffsol-triage list: {tc.describe_exit(proc.returncode)}: {tc.tail(proc.stderr, 400).strip()}"
    doc = tc.parse_json_output(proc.stdout)
    if doc is None or doc.get("schema") != DIFFSOL_LIST_SCHEMA or not isinstance(doc.get("problems"), list):
        return None, f"diffsol-triage list printed no {DIFFSOL_LIST_SCHEMA} document"
    return doc, None


def diffsol_definition_mismatch(problem: dict, spec: dict) -> list[str]:
    """Differences between the CORPUS (+ triage-list) problem and the harness's definition, compared exactly."""
    reasons = []
    if spec.get("dimension") != problem.get("dimension"):
        reasons.append(f"dimension {spec.get('dimension')!r} != {problem.get('dimension')!r}")
    theirs = [float(x) for x in problem.get("t_span", [])]
    if [float(x) for x in spec.get("t_span", [])] != theirs:
        reasons.append(f"t_span {spec.get('t_span')!r} != {problem.get('t_span')!r}")
    if not tc.finite(spec.get("atol_scale")) or float(spec["atol_scale"]) != float(problem["atol_scale"]):
        reasons.append(f"atol_scale {spec.get('atol_scale')!r} != {problem.get('atol_scale')!r}")
    if "breakpoints" in problem and [float(x) for x in spec.get("breakpoints") or []] != \
            [float(x) for x in problem.get("breakpoints") or []]:
        reasons.append(f"breakpoints {spec.get('breakpoints')!r} != {problem.get('breakpoints')!r}")
    return reasons


def process_abort(row: dict, kind: str, code: int | None, stderr, stdout, message: str) -> dict:
    row.update(status="process_abort", error=message, success=False)
    row["process"] = {"kind": kind, "exit_status": code,
                      "signal": tc.describe_exit(code) if code is not None and code < 0 else None,
                      "stderr_tail": tc.tail(stderr), "stdout_tail": tc.tail(stdout)}
    return row


DIFFSOL_COPIED_KEYS = ("status", "error", "success", "attempts", "accepted_steps", "rejected_steps",
                       "output_times", "states_hex", "counters", "extra_work", "counter_semantics",
                       "deterministic", "repetitions", "h0")


def diffsol_cell(cell: dict, problem: dict, campaign: str, prov: dict, binary: str | None,
                 binary_sha: str | None, listing: dict | None, listing_reason: str | None, repetitions: int,
                 timeout: float | None) -> dict:
    row = base_row(cell, problem, campaign, prov, binary_sha, repetitions)
    row["h0"] = None  # Diffsol chooses its own first step (extra_work.h0_policy)
    if listing is None:
        return unavailable(row, listing_reason or "no diffsol-triage list document")
    specs = {p.get("id"): p for p in listing["problems"]}
    spec = specs.get(cell["problem"])
    if spec is None:
        return unavailable(row, f"problem not implemented in the Diffsol harness (it has {sorted(specs)})")
    mismatch = diffsol_definition_mismatch(problem, spec)
    if mismatch:
        return unavailable(row, "Diffsol harness definition differs from CORPUS: " + "; ".join(mismatch))
    if cell["h0_scale"] != NOMINAL_H0:
        return unavailable(row, "Diffsol chooses its own first step; only the nominal h0_scale 1 cell exists")
    command = [binary, "run", "--problem", cell["problem"], "--method", DIFFSOL_ARMS[cell["arm"]], "--rtol",
               tc.fmt_float(cell["rtol"]), "--grid", cell["grid"], "--campaign", campaign, "--repetitions",
               str(repetitions)]
    try:
        proc = subprocess.run(command, capture_output=True, text=True, env=tc.provenance_env(prov), timeout=timeout)
    except subprocess.TimeoutExpired as exc:
        return process_abort(row, "timeout", None, exc.stderr, exc.stdout, f"timeout after {timeout} s")
    if proc.returncode == 2:
        return process_abort(row, "usage_exit_2", 2, proc.stderr, proc.stdout,
                             "exit status 2 (diffsol-triage refused the cell arguments)")
    if proc.returncode != 0:
        return process_abort(row, "abort", proc.returncode, proc.stderr, proc.stdout,
                             tc.describe_exit(proc.returncode))
    upstream = tc.parse_json_output(proc.stdout)
    if upstream is None or upstream.get("schema") != tc.ROW_SCHEMA or set(upstream) != set(tc.ROW_KEYS):
        return process_abort(row, "unparseable_row", 0, proc.stderr, proc.stdout,
                             f"exit 0 without a {tc.ROW_SCHEMA} row with the contract key set")
    wrong = [f"{key} {upstream.get(key)!r} != {row[key]!r}" for key in ("campaign", "arm", "problem", "rtol", "grid",
                                                                     "h0_scale", "dimension", "tier")
             if upstream.get(key) != row[key]]
    if wrong:
        return process_abort(row, "cell_mismatch", 0, proc.stderr, proc.stdout,
                             "row does not match the requested cell: " + "; ".join(wrong))
    if not tc.finite(upstream.get("atol")) or float(upstream["atol"]) != row["atol"]:
        return unavailable(row, f"diffsol atol {upstream.get('atol')!r} != cell atol {row['atol']!r}",
                           upstream.get("extra_work"))
    for key in DIFFSOL_COPIED_KEYS:
        row[key] = upstream[key]
    reported_sha = (upstream.get("binary") or {}).get("sha256")
    if reported_sha != binary_sha:
        row.update(status="error", success=False,
                   error=f"binary identity: the row reports sha256 {reported_sha!r}, the file has {binary_sha!r}")
        return row
    if row["status"] == "ok":
        times = tc.grid_times(problem["t_span"], cell["grid"])
        states = row["states_hex"]
        if row["output_times"] != times or not isinstance(states, list) or len(states) != len(times) or \
                any(not isinstance(s, list) or len(s) != problem.get("dimension") for s in states):
            row.update(status="error", success=False,
                       error="diffsol row output times or states differ from the requested grid (bitwise)")
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


def run_one(cell, problem, args_dict, prov, driver_sha):
    arm = cell["arm"]
    if arm in SCIPY_ARMS:
        return scipy_cell(cell, problem, args_dict["campaign"], prov, args_dict["repetitions"])
    if arm in NATIVE_ARMS:
        return native_cell(cell, problem, args_dict["campaign"], prov, args_dict["native_driver"], driver_sha,
                           args_dict["repetitions"])
    return diffsol_cell(cell, problem, args_dict["campaign"], prov, args_dict["diffsol_binary"],
                        args_dict["diffsol_sha"], args_dict["diffsol_listing"], args_dict["diffsol_listing_reason"],
                        args_dict["repetitions"], args_dict["diffsol_timeout"])


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--list", type=Path, help="triage-list JSON (adds jvp/breakpoints/h0_base of problems)")
    parser.add_argument("--output", required=True, type=Path, help="EXTERNAL.jsonl (appended; resumable)")
    parser.add_argument("--campaign", required=True)
    parser.add_argument("--external-arm", action="append", choices=EXTERNAL_ARMS,
                        help="external arm to run (repeatable; default: all)")
    parser.add_argument("--native-driver", help="tools/native_stiff build output `native_stiff`")
    parser.add_argument("--diffsol-binary", help="tools/benchmarks/diffsol/triage build output `diffsol-triage`")
    parser.add_argument("--diffsol-timeout", type=float, default=DEFAULT_DIFFSOL_TIMEOUT,
                        help=f"seconds per Diffsol cell (default {DEFAULT_DIFFSOL_TIMEOUT:g}); 0 = no limit")
    parser.add_argument("--repetitions", type=int, default=1, help="runs per cell for the determinism flag")
    parser.add_argument("-j", "--jobs", type=int, default=1, help="parallel worker processes")
    parser.add_argument("--force", action="store_true")
    tc.add_selection_arguments(parser)
    tc.add_state_arguments(parser)
    args = parser.parse_args(argv)
    try:
        corpus = tc.load_json(args.corpus)
        list_doc = tc.load_json(args.list) if args.list else None
        externalize = tc.StateExternalizer(args.states_dir, args.inline_states_max_dimension,
                                           tc.load_references(args.references), tc.corpus_problems(corpus))
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
    diffsol = args.diffsol_binary if args.diffsol_binary and Path(args.diffsol_binary).is_file() else None
    diffsol_sha = tc.sha256_file(Path(diffsol)) if diffsol else None
    wants_diffsol = any(c["arm"] in DIFFSOL_ARMS for c in cells)
    listing, listing_reason = diffsol_listing(diffsol) if wants_diffsol else (None, None)
    if diffsol is None and args.diffsol_binary:
        listing_reason = f"Diffsol harness binary {args.diffsol_binary!r} not found ({DIFFSOL_BUILD_HINT})"
    if args.force and args.output.exists():
        args.output.unlink()
    done = {tc.cell_key(r) for r in tc.load_jsonl(args.output)} if args.output.exists() else set()
    todo = [c for c in cells if (args.campaign, c["arm"], c["problem"], c["rtol"], c["grid"], c["h0_scale"])
            not in done]
    print(f"{len(cells)} planned external cells, {len(todo)} to run", file=sys.stderr)
    prov = tc.git_provenance()
    args_dict = {"campaign": args.campaign, "repetitions": max(1, args.repetitions), "native_driver": driver,
                 "diffsol_binary": diffsol, "diffsol_sha": diffsol_sha, "diffsol_listing": listing,
                 "diffsol_listing_reason": listing_reason,
                 "diffsol_timeout": args.diffsol_timeout if args.diffsol_timeout and args.diffsol_timeout > 0 else None}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    appender = tc.OrderedAppender(args.output, len(todo))
    pool_type = concurrent.futures.ProcessPoolExecutor if args.jobs > 1 else concurrent.futures.ThreadPoolExecutor
    try:
        with pool_type(max_workers=max(1, args.jobs)) as pool:
            futures = {pool.submit(run_one, c, problems[c["problem"]], args_dict, prov, driver_sha): i
                       for i, c in enumerate(todo)}
            for future in concurrent.futures.as_completed(futures):
                row = future.result()
                print(f"{row['arm']} {row['problem']} rtol={row['rtol']:g} {row['grid']} h0x{row['h0_scale']:g}: "
                      f"{row['status']} {row['error'] or ''}", file=sys.stderr, flush=True)
                appender.add(futures[future], externalize(row))
    finally:
        appender.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
