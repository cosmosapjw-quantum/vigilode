#!/usr/bin/env python3
"""Gate of research node research/ct01_controller_grid_holdout_20261010 (CT01; see its PREREGISTRATION.md).

Inputs: RUNS.json (export_runs of crates/rodas5p-cli/tests/controller_grid_holdout.rs: the references and arms I,
PREDcap, PREDcap2 on every cell) and the NATIVE.json references (immutable, AS03 pin).
Cells: the dense fast driver on van-der-pol-mu1000, hires, robertson, brusselator-1d-50 with ALG05's quarter-decade
ladders, the interior grid t_k = t0 + k (tf - t0)/40 (k = 1..40), the gated seeds {2e-6, 7e-6, 2e-5, 7e-5, 2e-4,
7e-4} and the anchor seed 3e-6 (reported only). The seeds are a fixed deterministic corpus, not random samples.

Error: the grid error is max over the 40 grid points of max_i |y_i - r_i| / max(|r_i|, 1e-10) against the reference
trajectory (arm I, h0 = 1e-6, rtol 1e-13, on the grid); the endpoint error is the same metric at tf (reported).
The reference uncertainty u(problem) is the grid error of the check (rtol 1e-12) against the reference.

Evidence (INVALID, exit 2) before any gate:
- AS03: tools/evidence_schema_v2.py primitives (strict JSON, the NATIVE.json SHA-256 pin, exact key sets, raw
  duplicate detection, typed finite numbers, canonical binary64 hex, vector lengths) applied to this schema; and
  the telemetry bound to its run (clipped landings = output_clipped_steps, updates = attempts).
- Row set: every (problem, seed, rtol, arm) exactly once (92 rungs x 7 seeds x 3 arms = 1932), registered values.
- Every grid and endpoint error (runs, references, uncertainty, NATIVE difference) recomputed from the state bits
  equals the exported one.
- The telemetry parity tests exist (crates/rodas5p-cli/tests/ct01_controller_telemetry_parity.rs and the parity
  test of controller_grid_holdout.rs).
- No successful run (or reference) has output_clipped_steps = 0.
- At most 5% of the gated cells are reference-limited: a gated cell (problem, gated seed, rtol) is reference-limited
  when the grid error of arm I or PREDcap2 (the compared arms; the smaller successful one) is below 100 u. Such
  cells are excluded from every gated statistic (both arms) and counted.

Scoring: ALG05's definitions (tools/alg05_controller_v2_check.py: OLS frontier fit of log10(attempts) on
log10(error) over the whole ladder, per-seed ratios, median over seeds, in-range half-decade E, the calibration fit)
imported unchanged and applied to the grid error over the 6 gated seeds.

Gate (PREDcap2 against I, gated seeds), PASS iff all hold:
1. Matched-error work: van der Pol frontier ratio <= 0.90 at >= 3 of E in {1e-3, 1e-4, 1e-5, 1e-6, 3e-7}
   (extrapolated E excluded); hires, robertson, brusselator-1d-50: worst in-range half-decade ratio <= 1.06.
2. Worst-seed guard: for every problem and in-range E, the maximum over the 6 seeds of the per-seed ratio <= 1.20.
3. Rejections: pooled van der Pol rejection fraction of PREDcap2 <= 0.5 x I's.
4. Relative calibration, per problem: (a) median over seeds of max over the rtol range of fit_PREDcap2 / fit_I
   <= 2.0; (b) median over the non-excluded cells of err_PREDcap2 / err_I in [0.5, 2.0].
5. Catastrophic-cell guard: no gated cell with err_PREDcap2 / rtol > 10 and err_PREDcap2 > 3 err_I; no gated cell
   with PREDcap2 failing (success false) where I succeeds.
Everything else is FAIL (exit 1), with every number preserved. PASS exits 0. Counted work only; no wall-time claim.
The output file is immutable.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from collections import defaultdict
from pathlib import Path

sys.dont_write_bytecode = True
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)
import alg05_controller_v2_check as alg05  # noqa: E402  (ALG05's scoring definitions, unchanged)

SCHEMA = "vigilode-ct01-controller-grid-check-v1"
RUNS_SCHEMA = "vigilode-ct01-runs-v1"
ERROR_FLOOR = 1.0e-10
PROBLEMS = ("van-der-pol-mu1000", "hires", "robertson", "brusselator-1d-50")
VDP = "van-der-pol-mu1000"
OTHER_PROBLEMS = ("hires", "robertson", "brusselator-1d-50")
VDP_E = alg05.VDP_E
SPANS = {"van-der-pol-mu1000": (0.0, 2000.0), "hires": (0.0, 321.8122), "robertson": (0.0, 40.0),
         "brusselator-1d-50": (0.0, 10.0)}
ATOL_SCALE = {"van-der-pol-mu1000": 1.0, "hires": 1.0e-4, "robertson": 1.0e-4, "brusselator-1d-50": 1.0}
LAST_QUARTER = {"van-der-pol-mu1000": 28, "hires": 40, "robertson": 40, "brusselator-1d-50": 28}
FIRST_QUARTER = 12
ARMS = ("I", "PREDcap", "PREDcap2")
GATED = "PREDcap2"
REFERENCE = "I"
GATED_SEEDS = (2.0e-6, 7.0e-6, 2.0e-5, 7.0e-5, 2.0e-4, 7.0e-4)
ANCHOR_SEED = 3.0e-6
SEEDS = (2.0e-6, 3.0e-6, 7.0e-6, 2.0e-5, 7.0e-5, 2.0e-4, 7.0e-4)
GRID_POINTS = 40
REFERENCE_SEED = 1.0e-6
REFERENCE_RTOL = 1.0e-13
CHECK_RTOL = 1.0e-12
MAX_ATTEMPTS = 1000000
RUNS_TOTAL = 1932
GATED_CELLS = 552
REFERENCE_LIMITED_FACTOR = 100.0
REFERENCE_LIMITED_MAX_FRACTION = 0.05
ITEM1_VDP_R = 0.90
ITEM1_VDP_COUNT = 3
ITEM1_OTHER_R = 1.06
ITEM2_WORST_SEED_R = 1.20
ITEM3_FACTOR = 0.5
ITEM4A_FIT_RATIO = 2.0
ITEM4B_BAND = (0.5, 2.0)
ITEM5_ERR_OVER_RTOL = 10.0
ITEM5_ERR_RATIO = 3.0
REPORT_ERR_OVER_RTOL = 10.0
ERROR_METRIC = ("max over the 40 grid points of max_i |y_i - r_i| / max(|r_i|, 1e-10) against the reference "
                "trajectory at the same points")
ENDPOINT_METRIC = "max_i |y_i - r_i| / max(|r_i|, 1e-10) at tf against the reference"
EVENTS = ("clipped_landings", "sliver_landings", "sliver_landings_while_rejection_pending",
          "informative_clipped_landings_after_rejection", "zero_error_accepts_while_rejection_pending",
          "accepted_next_request_exceeds_trial")
TELEMETRY_KEYS = frozenset(EVENTS) | {"updates", "rejection_pending_at_end"}
DIVERGENCE_KEYS = frozenset(("update", "cause", "pending_before", "previous_attempt_accepted", "events_at_update",
                             "events_through_update"))
DIVERGENCE_THROUGH_KEYS = frozenset(("sliver_landings_while_rejection_pending",
                                     "zero_error_accepts_while_rejection_pending",
                                     "informative_clipped_landings_after_rejection", "sliver_landings",
                                     "clipped_landings", "accepted_next_request_exceeds_trial"))
DIVERGENCE_CAUSES = ("zero_error_while_pending", "pending_kept_by_sliver", "other")
KEY_FIELDS = frozenset(("problem", "arm", "rtol", "atol", "seed"))
OK_ROW_KEYS = KEY_FIELDS | {
    "ok", "success", "message", "attempts", "accepted", "rejected", "jacobian_reuses", "internal_steps",
    "output_clipped_steps", "rhs_evaluations", "lu_factorizations", "jacobian_builds", "linear_solve_failures",
    "nonfinite_step_failures", "counters", "grid_t", "grid_states", "telemetry", "grid_error", "endpoint_error"}
RUN_ROW_KEYS = OK_ROW_KEYS | {"first_difference_from_PREDcap"}
FAILED_ROW_KEYS = KEY_FIELDS | {"ok", "success", "error_message"}
FAILED_RUN_ROW_KEYS = FAILED_ROW_KEYS | {"first_difference_from_PREDcap"}
ROW_INTS = ("attempts", "accepted", "rejected", "jacobian_reuses", "internal_steps", "output_clipped_steps",
            "rhs_evaluations", "lu_factorizations", "jacobian_builds", "linear_solve_failures",
            "nonfinite_step_failures")
HEADER_KEYS = frozenset(("schema", "driver", "max_attempts", "grid_points", "gated_seeds", "anchor_seed", "seeds",
                         "arms", "reference", "error_metric", "endpoint_metric", "telemetry_events", "references",
                         "rows"))
HEADER = {"schema": RUNS_SCHEMA, "driver": "integrate_rodas5p_fast_observed_with_telemetry",
          "max_attempts": MAX_ATTEMPTS, "grid_points": GRID_POINTS, "gated_seeds": list(GATED_SEEDS),
          "anchor_seed": ANCHOR_SEED, "seeds": list(SEEDS), "arms": list(ARMS),
          "reference": {"arm": "I", "seed": REFERENCE_SEED, "rtol": REFERENCE_RTOL, "check_rtol": CHECK_RTOL},
          "error_metric": ERROR_METRIC, "endpoint_metric": ENDPOINT_METRIC, "telemetry_events": list(EVENTS)}
REFERENCE_RECORD_KEYS = frozenset(("reference", "check", "uncertainty", "native_endpoint_difference"))
PARITY_TESTS = {
    "crates/rodas5p-cli/tests/ct01_controller_telemetry_parity.rs": (
        "telemetry_on_and_off_runs_are_bitwise_identical", "traced_decisions_replay_through_the_controller"),
    "crates/rodas5p-cli/tests/controller_grid_holdout.rs": ("telemetry_parity_on_registered_cells",),
}


# ------------------------------------------------------------------------------------------- basics
def quarter_decade(q: int) -> float:
    """10^(-q/4); whole decades are the exact literals (as the export)."""
    return float(f"1e-{q // 4}") if q % 4 == 0 else 10.0 ** (-q / 4)


def rung(problem: str, rtol: float) -> int | None:
    """The quarter-decade index of a registered rtol of the problem's ladder, else None."""
    if not (isinstance(rtol, float) and rtol > 0.0 and math.isfinite(rtol)):
        return None
    q = round(-4.0 * math.log10(rtol))
    if not FIRST_QUARTER <= q <= LAST_QUARTER[problem]:
        return None
    return q if abs(rtol / quarter_decade(q) - 1.0) <= 1.0e-14 else None


def grid_times(problem: str) -> list[float]:
    t0, tf = SPANS[problem]
    return [t0 + k * (tf - t0) / GRID_POINTS for k in range(1, GRID_POINTS + 1)]


def point_error(y: list[float], r: list[float]) -> float:
    evidence.require_metric_pair(y, r)
    return max(abs(a - b) / max(abs(b), ERROR_FLOOR) for a, b in zip(y, r))


def grid_error(states: list[list[float]], reference: list[list[float]]) -> float:
    if len(states) != GRID_POINTS or len(reference) != GRID_POINTS:
        raise evidence.ValidationError([f"grid error needs {GRID_POINTS} points ({len(states)}, {len(reference)})"])
    return max(point_error(y, r) for y, r in zip(states, reference))


median = alg05.median


# ------------------------------------------------------------------------------------------ evidence
def _row(c, path: str, r, refs_dim: dict, kind: str, decoded: dict) -> None:
    """Type one exported run (kind: 'run' for a cell row, 'reference' for a reference/check row)."""
    if not c.dict_(path, r) or not c.bool_(f"{path}.ok", r.get("ok")):
        return
    ok = r["ok"]
    expected = (RUN_ROW_KEYS if ok else FAILED_RUN_ROW_KEYS) if kind == "run" else (
        OK_ROW_KEYS if ok else FAILED_ROW_KEYS)
    if not c.keys(path, r, expected):
        return
    problem = r["problem"]
    if problem not in PROBLEMS:
        c.fail(f"{path}.problem", f"unregistered problem {problem!r}")
        return
    if r["arm"] not in ARMS:
        c.fail(f"{path}.arm", f"unregistered arm {r['arm']!r}")
    if c.float_(f"{path}.rtol", r["rtol"], positive=True) and c.float_(f"{path}.atol", r["atol"], positive=True):
        if r["atol"] != r["rtol"] * ATOL_SCALE[problem]:
            c.fail(f"{path}.atol", f"atol {r['atol']!r} != rtol x atol_scale {r['rtol'] * ATOL_SCALE[problem]!r}")
    c.float_(f"{path}.seed", r["seed"], positive=True)
    c.bool_(f"{path}.success", r["success"])
    if kind == "run":
        div = r["first_difference_from_PREDcap"]
        if r.get("arm") != GATED and div is not None:
            c.fail(f"{path}.first_difference_from_PREDcap", "must be null outside PREDcap2")
        if div is not None and c.keys(f"{path}.first_difference_from_PREDcap", div, DIVERGENCE_KEYS):
            p = f"{path}.first_difference_from_PREDcap"
            c.int_(f"{p}.update", div["update"], minimum=1)
            if div["cause"] not in DIVERGENCE_CAUSES:
                c.fail(f"{p}.cause", f"unregistered cause {div['cause']!r}")
            c.bool_(f"{p}.pending_before", div["pending_before"])
            c.bool_(f"{p}.previous_attempt_accepted", div["previous_attempt_accepted"])
            if c.list_(f"{p}.events_at_update", div["events_at_update"]):
                for e in div["events_at_update"]:
                    if e not in EVENTS:
                        c.fail(f"{p}.events_at_update", f"unregistered event {e!r}")
            if c.keys(f"{p}.events_through_update", div["events_through_update"], DIVERGENCE_THROUGH_KEYS):
                for k, v in div["events_through_update"].items():
                    c.int_(f"{p}.events_through_update.{k}", v)
    if not ok:
        c.str_(f"{path}.error_message", r["error_message"], nonempty=False)
        if r["success"] is not False:
            c.fail(f"{path}.success", "a failed driver call cannot succeed")
        return
    c.str_(f"{path}.message", r["message"], nonempty=False)
    for k in ROW_INTS:
        c.int_(f"{path}.{k}", r[k])
    c.counters(f"{path}.counters", r["counters"], evidence.COUNTERS_DENSE)
    tel = r["telemetry"]
    if c.keys(f"{path}.telemetry", tel, TELEMETRY_KEYS):
        for k in EVENTS + ("updates",):
            c.int_(f"{path}.telemetry.{k}", tel[k])
        c.bool_(f"{path}.telemetry.rejection_pending_at_end", tel["rejection_pending_at_end"])
        # The telemetry is bound to its run.
        if type(tel["clipped_landings"]) is int and tel["clipped_landings"] != r["output_clipped_steps"]:
            c.fail(f"{path}.telemetry.clipped_landings", f"{tel['clipped_landings']} != output_clipped_steps "
                                                         f"{r['output_clipped_steps']}")
        if type(tel["updates"]) is int and tel["updates"] != r["attempts"]:
            c.fail(f"{path}.telemetry.updates", f"{tel['updates']} != attempts {r['attempts']}")
    dim = refs_dim[problem]
    times = c.hex_vector(f"{path}.grid_t", r["grid_t"]) if r["grid_t"] else []
    if not c.list_(f"{path}.grid_states", r["grid_states"]):
        return
    states = []
    for i, s in enumerate(r["grid_states"]):
        states.append(c.hex_vector(f"{path}.grid_states[{i}]", s, dim))
    if times is not None and len(r["grid_t"]) != len(r["grid_states"]):
        c.fail(f"{path}.grid_t", f"{len(r['grid_t'])} times for {len(r['grid_states'])} states")
    if r["success"] is True:
        if len(r["grid_states"]) != GRID_POINTS:
            c.fail(f"{path}.grid_states", f"a successful run has {len(r['grid_states'])} != {GRID_POINTS} points")
        elif times is not None and times != grid_times(problem):
            c.fail(f"{path}.grid_t", "the grid times are not the registered grid t0 + k (tf - t0)/40")
        for k in ("grid_error", "endpoint_error"):
            c.float_(f"{path}.{k}", r[k], minimum=0.0)
    else:
        for k in ("grid_error", "endpoint_error"):
            if r[k] is not None:
                c.fail(f"{path}.{k}", "must be null for an unsuccessful run")
        if times is not None and times != grid_times(problem)[:len(times)]:
            c.fail(f"{path}.grid_t", "the reached grid times are not the registered grid")
    if all(s is not None for s in states):
        decoded[id(r)] = states


def validate(runs_path: Path, native_path: Path, repo_root: Path, pins=evidence.REGISTERED_SHA256):
    """AS03 validation plus the registered validity conditions; raises evidence.ValidationError (INVALID).
    Returns (docs, digests, derived)."""
    docs, digests, _ = evidence._load_all([("runs", runs_path, None),
                                           ("native", native_path, None if pins is None else pins["native"])])
    runs, native = docs["runs"], docs["native"]
    c = evidence._Checks()
    if not (c.keys("runs", runs, HEADER_KEYS) and c.dict_("native", native)
            and c.dict_("native.references", native.get("references"))):
        c.done()
    for k, v in HEADER.items():
        c.equal(f"runs.{k}", runs[k], v)
    # NATIVE references (as AS03's ALG05 rule).
    refs_native: dict[str, list[float]] = {}
    for problem in PROBLEMS:
        ref = native["references"].get(problem)
        p = f"native.references.{problem}"
        if ref is None:
            c.fail("native.references", f"missing reference for {problem!r}")
            continue
        if c.keys(p, ref, evidence.NATIVE_REFERENCE_KEYS):
            c.str_(f"{p}.method", ref["method"])
            for k in ("uncertainty", "lsoda_rtol_1e-12_difference", "seconds"):
                c.float_(f"{p}.{k}", ref[k], minimum=0.0)
            y = c.float_vector(f"{p}.final_state", ref["final_state"])
            if y is not None:
                refs_native[problem] = y
    if not c.list_("runs.rows", runs["rows"], nonempty=True) or not c.dict_("runs.references", runs["references"]):
        c.done()
    c.done()
    dims = {p: len(v) for p, v in refs_native.items()}

    # [schema] references.
    decoded: dict[int, list] = {}
    if set(runs["references"]) != set(PROBLEMS):
        c.fail("runs.references", f"problem set {sorted(runs['references'])} != registered {sorted(PROBLEMS)}")
        c.done()
    for problem in PROBLEMS:
        rec = runs["references"][problem]
        p = f"runs.references.{problem}"
        if not c.keys(p, rec, REFERENCE_RECORD_KEYS):
            continue
        for role, rtol in (("reference", REFERENCE_RTOL), ("check", CHECK_RTOL)):
            r = rec[role]
            _row(c, f"{p}.{role}", r, dims, "reference", decoded)
            if isinstance(r, dict):
                for k, v in (("problem", problem), ("arm", "I"), ("seed", REFERENCE_SEED), ("rtol", rtol)):
                    c.equal(f"{p}.{role}.{k}", r.get(k), v)
                if r.get("ok") is not True or r.get("success") is not True:
                    c.fail(f"{p}.{role}", "the reference runs must succeed")
        c.float_(f"{p}.uncertainty", rec["uncertainty"], minimum=0.0)
        c.float_(f"{p}.native_endpoint_difference", rec["native_endpoint_difference"], minimum=0.0)

    # [row-set] typed key fields, raw duplicates, registered values, completeness.
    rows = runs["rows"]
    fields = {"problem": "str", "arm": "str", "rtol": "float+", "seed": "float+"}
    index = evidence._rows(c, "runs.rows", rows, fields, lambda r: (r["problem"], r["seed"], r["rtol"], r["arm"]))
    c.done()
    for k, r in index.items():
        path = f"runs.rows{list(k)}"
        if r["problem"] in PROBLEMS and rung(r["problem"], r["rtol"]) is None:
            c.fail(f"{path}.rtol", f"not a rung of the registered ladder: {r['rtol']!r}")
        if r["seed"] not in SEEDS:
            c.fail(f"{path}.seed", f"unregistered seed {r['seed']!r}")
        _row(c, path, r, dims, "run", decoded)
    expected = set()
    rtol_of = {}
    for problem in PROBLEMS:
        for q in range(FIRST_QUARTER, LAST_QUARTER[problem] + 1):
            for seed in SEEDS:
                for arm in ARMS:
                    expected.add((problem, seed, q, arm))
    seen = set()
    for (problem, seed, rtol, arm) in index:
        q = rung(problem, rtol) if problem in PROBLEMS else None
        if q is None:
            continue
        seen.add((problem, seed, q, arm))
        prior = rtol_of.setdefault((problem, q), rtol)
        if prior != rtol:
            c.fail("runs.rows", f"{problem} rung {q} has two rtol values {prior!r} and {rtol!r}")
    missing, extra = expected - seen, seen - expected
    if missing or extra or len(rows) != RUNS_TOTAL or len(index) != RUNS_TOTAL:
        c.fail("runs.rows", f"[row-set] incomplete: {len(rows)} rows ({RUNS_TOTAL} registered), missing "
                            f"{len(missing)} (first {sorted(missing)[:3]}), unexpected {len(extra)}")
    c.done()

    # [error-recomputation] every exported error from the state bits.
    reference_states = {}
    uncertainty = {}
    for problem in PROBLEMS:
        rec = runs["references"][problem]
        ref_states = decoded[id(rec["reference"])]
        reference_states[problem] = ref_states
        for role in ("reference", "check"):
            r = rec[role]
            g = grid_error(decoded[id(r)], ref_states)
            e = point_error(decoded[id(r)][-1], ref_states[-1])
            if g != r["grid_error"] or e != r["endpoint_error"]:
                c.fail(f"runs.references.{problem}.{role}", f"[error-recomputation] recorded ({r['grid_error']!r}, "
                                                            f"{r['endpoint_error']!r}) != recomputed ({g!r}, {e!r})")
        u = grid_error(decoded[id(rec["check"])], ref_states)
        if rec["uncertainty"] != u:
            c.fail(f"runs.references.{problem}.uncertainty", f"[error-recomputation] {rec['uncertainty']!r} != {u!r}")
        d = point_error(ref_states[-1], refs_native[problem])
        if rec["native_endpoint_difference"] != d:
            c.fail(f"runs.references.{problem}.native_endpoint_difference",
                   f"[error-recomputation] {rec['native_endpoint_difference']!r} != {d!r}")
        uncertainty[problem] = u
    for k, r in index.items():
        if r.get("ok") is True and r.get("success") is True and id(r) in decoded:
            states = decoded[id(r)]
            g = grid_error(states, reference_states[r["problem"]])
            e = point_error(states[-1], reference_states[r["problem"]][-1])
            if g != r["grid_error"] or e != r["endpoint_error"]:
                c.fail(f"runs.rows{list(k)}", f"[error-recomputation] recorded ({r['grid_error']!r}, "
                                              f"{r['endpoint_error']!r}) != recomputed ({g!r}, {e!r})")
    c.done()

    # [parity-test] the registered telemetry parity tests exist.
    parity = parity_tests(repo_root)
    for f, tests in parity.items():
        for name, present in tests["present"].items():
            if not present:
                c.fail("parity-test", f"[parity-test] missing #[test] fn {name} in {f}")

    # [grid-inactive] every successful run clipped at least one output landing.
    all_rows = [(f"runs.rows{list(k)}", r) for k, r in index.items()] + [
        (f"runs.references.{p}.{role}", runs["references"][p][role]) for p in PROBLEMS for role in ("reference", "check")]
    for path, r in all_rows:
        if r.get("ok") is True and r.get("success") is True and r["output_clipped_steps"] == 0:
            c.fail(path, "[grid-inactive] a successful run has output_clipped_steps = 0")

    # [reference-limited] at most 5% of the gated cells.
    limited = reference_limited(index, uncertainty)
    fraction = len(limited) / GATED_CELLS
    if fraction > REFERENCE_LIMITED_MAX_FRACTION:
        c.fail("runs.rows", f"[reference-limited] {len(limited)} of {GATED_CELLS} gated cells ({fraction:.4f}) are "
                            f"reference-limited (> {REFERENCE_LIMITED_MAX_FRACTION})")
    c.done()
    derived = {"index": index, "uncertainty": uncertainty, "reference_limited": limited, "parity": parity,
               "rtol_of": rtol_of}
    return docs, digests, derived


def parity_tests(repo_root: Path) -> dict:
    out = {}
    for f, names in PARITY_TESTS.items():
        path = repo_root / f
        text = path.read_text() if path.exists() else ""
        out[f] = {"exists": path.exists(),
                  "sha256": evidence._sha256(path.read_bytes()) if path.exists() else None,
                  "present": {n: bool(re.search(r"#\[test\]\s*fn\s+" + n + r"\s*\(", text)) for n in names}}
    return out


def reference_limited(index: dict, uncertainty: dict) -> list[dict]:
    """Gated cells whose grid error (the smaller successful one of arms I and PREDcap2) is below 100 u."""
    out = []
    cells = defaultdict(dict)
    for (problem, seed, rtol, arm), r in index.items():
        if seed in GATED_SEEDS:
            cells[(problem, seed, rtol)][arm] = r
    for (problem, seed, rtol), arms in sorted(cells.items()):
        errors = [arms[a]["grid_error"] for a in (REFERENCE, GATED)
                  if a in arms and arms[a].get("ok") is True and arms[a].get("success") is True]
        if errors and min(errors) < REFERENCE_LIMITED_FACTOR * uncertainty[problem]:
            out.append({"problem": problem, "seed": seed, "rtol": rtol, "min_grid_error": min(errors),
                        "threshold": REFERENCE_LIMITED_FACTOR * uncertainty[problem]})
    return out


# ---------------------------------------------------------------------------------------------- gate
def scoring_rows(index: dict, error_key: str, excluded: set, seeds=GATED_SEEDS) -> list[dict]:
    """Rows of the given seeds with `error` = the chosen exported error (ALG05's scoring field), excluded cells
    (reference-limited) dropped for every arm."""
    out = []
    for (problem, seed, rtol, arm), r in index.items():
        if seed not in seeds or (problem, seed, rtol) in excluded:
            continue
        row = dict(r)
        row["error"] = r.get(error_key) if r.get("ok") else None
        out.append(row)
    return out


def group(rows: list[dict]) -> dict:
    groups = defaultdict(list)
    for r in rows:
        groups[(r["problem"], r["arm"], r["seed"])].append(r)
    for k in groups:
        groups[k].sort(key=lambda r: -r["rtol"])
    return groups


def cells_of(rows: list[dict]) -> dict:
    cells = defaultdict(dict)
    for r in rows:
        cells[(r["problem"], r["seed"], r["rtol"])][r["arm"]] = r
    return cells


def ok_success(r) -> bool:
    return r is not None and r.get("ok") is True and r.get("success") is True


def equal_rtol_ratios(cells: dict, problem: str, arm: str, ref: str = REFERENCE) -> list[float]:
    return [c[arm]["error"] / c[ref]["error"] for k, c in sorted(cells.items())
            if k[0] == problem and ok_success(c.get(arm)) and ok_success(c.get(ref))
            and c[ref]["error"] > 0.0 and c[arm]["error"] is not None]


def gate_items(rows: list[dict], gated: str, ref: str = REFERENCE, seeds=GATED_SEEDS) -> dict:
    """Items 1-5 of `gated` against `ref` on `rows` (already restricted to the seeds and non-excluded cells)."""
    groups = group(rows)
    cells = cells_of(rows)
    tables = {p: alg05.table(groups, p, gated, ref, seeds) for p in PROBLEMS}

    # 1. Matched-error work.
    vdp = tables[VDP]
    passing = [p["E"] for p in vdp["points"] if not p["extrapolated"] and p["R"] is not None
               and p["R"] <= ITEM1_VDP_R]
    others = {}
    for problem in OTHER_PROBLEMS:
        t = tables[problem]
        others[problem] = {
            "in_range_E": t["in_range_E"], "excluded_E": t["excluded_E"],
            "points": [{"E": p["E"], "R": p["R"], "R_range": p["R_range"], "reported_C": p["C"],
                        "reported_C_range": p["C_range"], "extrapolated": p["extrapolated"]} for p in t["points"]],
            "worst_R": t["worst_R"], "worst_R_E": t["worst_R_E"], "gm_R": t["gm_R"],
            "reported_worst_C": t["reported_worst_C"], "reported_gm_C": t["reported_gm_C"],
            "pass": bool(t["in_range_E"]) and t["worst_R"] is not None and t["worst_R"] <= ITEM1_OTHER_R,
        }
    item1 = {
        "item": "matched-error work: van der Pol frontier ratio <= 0.90 at >= 3 of E in {1e-3, 1e-4, 1e-5, 1e-6, "
                "3e-7}; hires, robertson, brusselator-1d-50 worst in-range half-decade frontier ratio <= 1.06 "
                "(median over the gated seeds; E outside either arm's measured range for some seed excluded)",
        "van_der_pol": {"points": [{"E": p["E"], "R": p["R"], "R_range": p["R_range"],
                                    "extrapolated": p["extrapolated"], "reported_C": p["C"],
                                    "per_seed": p["per_seed"]} for p in vdp["points"]],
                        "passing_E": passing, "pass": len(passing) >= ITEM1_VDP_COUNT},
        "others": others,
    }
    item1["pass"] = item1["van_der_pol"]["pass"] and all(v["pass"] for v in others.values())

    # 2. Worst-seed guard.
    guard = {}
    for problem in PROBLEMS:
        pts = [p for p in tables[problem]["points"] if not p["extrapolated"] and p["R"] is not None]
        per = [{"E": p["E"], "max_seed_R": max(s["R"] for s in p["per_seed"]),
                "argmax_seed": max(p["per_seed"], key=lambda s: s["R"])["seed"]} for p in pts]
        worst = max((x["max_seed_R"] for x in per), default=None)
        guard[problem] = {"points": per, "worst": worst,
                          "pass": bool(per) and worst is not None and worst <= ITEM2_WORST_SEED_R}
    item2 = {"item": "worst-seed guard: for every problem and in-range E, max over the 6 gated seeds of the per-seed "
                     "frontier ratio <= 1.20",
             "problems": guard, "pass": all(v["pass"] for v in guard.values())}

    # 3. Rejections.
    fi = alg05.rejection([r for r in rows if r["problem"] == VDP and r["arm"] == ref])
    fg = alg05.rejection([r for r in rows if r["problem"] == VDP and r["arm"] == gated])
    item3 = {"item": f"rejections: pooled van der Pol rejection fraction of {gated} <= 0.5 x {ref}'s",
             ref: fi, gated: fg,
             "ratio": fg["fraction"] / fi["fraction"] if fi["fraction"] else None,
             "pass": fi["fraction"] is not None and fg["fraction"] is not None
             and fg["fraction"] <= ITEM3_FACTOR * fi["fraction"]}

    # 4. Relative calibration.
    per_problem = {}
    for problem in PROBLEMS:
        fitc = alg05.calibration(groups, problem, gated, ref, seeds)
        ratios = equal_rtol_ratios(cells, problem, gated, ref)
        med = median(ratios)
        a_pass = fitc["median_max_ratio"] is not None and fitc["median_max_ratio"] <= ITEM4A_FIT_RATIO
        b_pass = med is not None and ITEM4B_BAND[0] <= med <= ITEM4B_BAND[1]
        per_problem[problem] = {
            "a_median_max_fit_ratio": fitc["median_max_ratio"], "a_per_seed": fitc["per_seed"], "a_pass": a_pass,
            "b_median_equal_rtol_ratio": med, "b_cells": len(ratios),
            "b_min": min(ratios) if ratios else None, "b_max": max(ratios) if ratios else None, "b_pass": b_pass,
            "pass": a_pass and b_pass}
    item4 = {"item": f"relative calibration per problem: (a) median over seeds of max over the rtol range of "
                     f"fit_{gated}/fit_{ref} <= 2.0; (b) median over the non-excluded cells of err_{gated}/err_{ref} "
                     f"in [0.5, 2.0]",
             "problems": per_problem, "pass": all(v["pass"] for v in per_problem.values())}

    # 5. Catastrophic-cell guard.
    catastrophic, failing = [], []
    for (problem, seed, rtol), c in sorted(cells.items()):
        g, i = c.get(gated), c.get(ref)
        if ok_success(g) and ok_success(i):
            q = g["error"] / rtol
            if q > ITEM5_ERR_OVER_RTOL and g["error"] > ITEM5_ERR_RATIO * i["error"]:
                catastrophic.append({"problem": problem, "seed": seed, "rtol": rtol, "err_over_rtol": q,
                                     "err_ratio": g["error"] / i["error"] if i["error"] > 0 else None})
        if ok_success(i) and not ok_success(g):
            failing.append({"problem": problem, "seed": seed, "rtol": rtol})
    item5 = {"item": f"catastrophic-cell guard: no gated cell with err_{gated}/rtol > 10 and err_{gated} > 3 err_{ref}; "
                     f"no gated cell with {gated} failing where {ref} succeeds",
             "catastrophic_cells": catastrophic, "failing_cells": failing,
             "pass": not catastrophic and not failing}
    return {"1_matched_error_work": item1, "2_worst_seed_guard": item2, "3_rejections": item3,
            "4_relative_calibration": item4, "5_catastrophic_cell_guard": item5}


# ------------------------------------------------------------------------------------------ reported
def fit_diagnostics(groups: dict, problem: str, arm: str, seed: float) -> dict:
    rows = groups.get((problem, arm, seed), [])
    use = [r for r in rows if alg05.usable(r)]
    out = {"n": len(use), "measured_error_range": [min(r["error"] for r in use), max(r["error"] for r in use)]
           if use else None}
    for name, pts in (("frontier", [(math.log10(r["error"]), math.log10(r["attempts"])) for r in use]),
                      ("calibration", [(math.log10(r["rtol"]), math.log10(r["error"])) for r in use])):
        f = alg05.ols(pts)
        if f is None:
            out[name] = None
            continue
        b, a = f
        rms = math.sqrt(sum((y - (a + b * x)) ** 2 for x, y in pts) / len(pts))
        out[name] = {"slope": b, "intercept": a, "residual_rms": rms}
    return out


def telemetry_totals(rows: list[dict]) -> dict:
    out = {}
    for arm in ARMS:
        per = {}
        for problem in PROBLEMS + ("all",):
            sel = [r for r in rows if r["arm"] == arm and (problem == "all" or r["problem"] == problem)
                   and r.get("ok")]
            per[problem] = {e: sum(r["telemetry"][e] for r in sel) for e in EVENTS}
            per[problem]["updates"] = sum(r["telemetry"]["updates"] for r in sel)
            per[problem]["runs"] = len(sel)
            per[problem]["runs_with"] = {e: sum(1 for r in sel if r["telemetry"][e] > 0) for e in EVENTS}
            per[problem]["runs_rejection_pending_at_end"] = sum(1 for r in sel
                                                                if r["telemetry"]["rejection_pending_at_end"])
        out[arm] = per
    return out


DIFF_FIELDS = ("success", "attempts", "accepted", "rejected", "jacobian_reuses", "rhs_evaluations",
               "output_clipped_steps", "grid_states", "grid_t", "counters", "telemetry")


def predcap_comparison(index: dict) -> dict:
    cells = defaultdict(dict)
    for (problem, seed, rtol, arm), r in index.items():
        cells[(problem, seed, rtol)][arm] = r
    differing, inconsistent = [], []
    causes = defaultdict(int)
    events_at = defaultdict(int)
    through = defaultdict(int)
    per_problem = {p: 0 for p in PROBLEMS}
    for (problem, seed, rtol), c in sorted(cells.items()):
        a, b = c["PREDcap"], c["PREDcap2"]
        differs = any(a.get(f) != b.get(f) for f in DIFF_FIELDS)
        div = b["first_difference_from_PREDcap"]
        if (div is not None) != differs:
            inconsistent.append({"problem": problem, "seed": seed, "rtol": rtol, "rows_differ": differs,
                                 "trace_differs": div is not None})
        if div is None:
            continue
        per_problem[problem] += 1
        causes[div["cause"]] += 1
        for e in div["events_at_update"]:
            events_at[e] += 1
        for e, n in div["events_through_update"].items():
            through[e] += int(n > 0)
        differing.append({"problem": problem, "seed": seed, "rtol": rtol, "gated_seed": seed in GATED_SEEDS,
                          "first_difference": div,
                          "PREDcap": {"attempts": a.get("attempts"), "rejected": a.get("rejected"),
                                      "grid_error": a.get("grid_error")},
                          "PREDcap2": {"attempts": b.get("attempts"), "rejected": b.get("rejected"),
                                       "grid_error": b.get("grid_error")}})
    return {"cells": len(cells), "differing_cells": len(differing), "differing_cells_per_problem": per_problem,
            "causes": dict(causes), "events_at_first_difference": dict(events_at),
            "cells_with_event_through_first_difference": dict(through),
            "row_vs_trace_inconsistencies": inconsistent, "differing": differing}


def cell_view(c: dict, rtol: float) -> dict:
    return {arm: ({"success": r.get("success"), "grid_error": r.get("grid_error"),
                   "err_over_rtol": r["grid_error"] / rtol if ok_success(r) else None,
                   "endpoint_error": r.get("endpoint_error"), "attempts": r.get("attempts"),
                   "rejected": r.get("rejected")} if r is not None else None) for arm, r in c.items()}


def high_cells(index: dict, seeds=SEEDS) -> list[dict]:
    """Every cell with grid err/rtol > 10 for any arm, with all arms and the neighbouring seeds' cells."""
    cells = defaultdict(dict)
    for (problem, seed, rtol, arm), r in index.items():
        cells[(problem, seed, rtol)][arm] = r
    out = []
    for (problem, seed, rtol), c in sorted(cells.items()):
        if seed not in seeds:
            continue
        arms_over = [a for a, r in c.items() if ok_success(r) and r["grid_error"] / rtol > REPORT_ERR_OVER_RTOL]
        if not arms_over:
            continue
        i = SEEDS.index(seed)
        neighbours = {str(s): cell_view(cells[(problem, s, rtol)], rtol)
                      for s in SEEDS[max(0, i - 1):i + 2] if s != seed}
        out.append({"problem": problem, "seed": seed, "anchor": seed == ANCHOR_SEED, "gated": seed in GATED_SEEDS,
                    "rtol": rtol, "arms_above_10": arms_over, "arms": cell_view(c, rtol),
                    "neighbouring_seeds": neighbours})
    return out


def error_over_rtol_summary(rows: list[dict]) -> dict:
    out = {}
    for arm in ARMS:
        per = {}
        for p in PROBLEMS:
            q = [r["error"] / r["rtol"] for r in rows if r["arm"] == arm and r["problem"] == p and alg05.usable(r)]
            per[p] = {"median": median(q), "max": max(q) if q else None,
                      "cells_above_2": sum(1 for x in q if x > 2.0), "cells_above_10": sum(1 for x in q if x > 10.0)}
        out[arm] = per
    return out


def equal_rtol_distributions(rows: list[dict]) -> dict:
    cells = cells_of(rows)
    out = {}
    for arm in ("PREDcap", "PREDcap2"):
        per = {}
        for problem in PROBLEMS:
            rs = equal_rtol_ratios(cells, problem, arm)
            per[problem] = {"median": median(rs), "min": min(rs) if rs else None, "max": max(rs) if rs else None,
                            "n": len(rs)}
        out[f"{arm}/I"] = per
    return out


def anchor_report(index: dict, excluded_none: set) -> dict:
    rows = scoring_rows(index, "grid_error", excluded_none, seeds=(ANCHOR_SEED,))
    groups = group(rows)
    cells = cells_of(rows)
    per_problem = {}
    for problem in PROBLEMS:
        per_problem[problem] = {
            "frontier_PREDcap2/I": alg05.table(groups, problem, "PREDcap2", "I", (ANCHOR_SEED,)),
            "calibration_PREDcap2/I": alg05.calibration(groups, problem, "PREDcap2", "I", (ANCHOR_SEED,)),
            "equal_rtol_PREDcap2/I": {"median": median(equal_rtol_ratios(cells, problem, "PREDcap2")),
                                      "max": max(equal_rtol_ratios(cells, problem, "PREDcap2"), default=None)},
            "rejection_fraction": {arm: alg05.rejection(groups[(problem, arm, ANCHOR_SEED)])["fraction"]
                                   for arm in ARMS},
            "cells": [{"rtol": k[2], **cell_view(c, k[2])} for k, c in sorted(cells.items()) if k[0] == problem],
        }
    return {"seed": ANCHOR_SEED, "note": "reported only; never gated and never excluded",
            "problems": per_problem, "cells_above_10": high_cells(index, seeds=(ANCHOR_SEED,))}


# ---------------------------------------------------------------------------------------------- main
def run(runs_path: Path, native_path: Path, output: Path, repo_root: Path) -> int:
    if output.exists():
        raise SystemExit(f"immutable output exists: {output}")
    try:
        docs, digests, derived = validate(runs_path, native_path, repo_root)
    except evidence.ValidationError as exc:
        return evidence.emit_invalid(output, SCHEMA, exc, {"runs": runs_path, "native": native_path})
    runs = docs["runs"]
    index = derived["index"]
    excluded = {(c["problem"], c["seed"], c["rtol"]) for c in derived["reference_limited"]}

    gated_rows = scoring_rows(index, "grid_error", excluded)
    gates = gate_items(gated_rows, GATED)
    verdict = "PASS" if all(g["pass"] for g in gates.values()) else "FAIL"

    # Reported, not gated.
    endpoint_rows = scoring_rows(index, "endpoint_error", excluded)
    endpoint_items = gate_items(endpoint_rows, GATED)
    unexcluded_rows = scoring_rows(index, "grid_error", set())
    unexcluded_items = gate_items(unexcluded_rows, GATED)
    predcap_items = gate_items(gated_rows, "PREDcap")
    groups = group(gated_rows)
    all_rows = scoring_rows(index, "grid_error", set(), seeds=SEEDS)
    all_groups = group(all_rows)
    diagnostics = {arm: {p: {str(s): fit_diagnostics(all_groups, p, arm, s) for s in SEEDS} for p in PROBLEMS}
                   for arm in ARMS}
    ratios = {}
    for arm, ref in (("PREDcap2", "I"), ("PREDcap", "I"), ("PREDcap2", "PREDcap")):
        ratios[f"{arm}/{ref}"] = {p: alg05.table(groups, p, arm, ref, GATED_SEEDS) for p in PROBLEMS}
    rejections = {arm: {p: alg05.rejection([r for r in gated_rows if r["problem"] == p and r["arm"] == arm])
                        for p in PROBLEMS} for arm in ARMS}
    references = {p: {"uncertainty": derived["uncertainty"][p],
                      "reference_limited_threshold": REFERENCE_LIMITED_FACTOR * derived["uncertainty"][p],
                      "native_endpoint_difference": runs["references"][p]["native_endpoint_difference"],
                      "native_uncertainty": docs["native"]["references"][p]["uncertainty"],
                      "reference_attempts": runs["references"][p]["reference"]["attempts"],
                      "check_attempts": runs["references"][p]["check"]["attempts"]} for p in PROBLEMS}
    rows = [r for r in index.values()]
    out = {
        "schema": SCHEMA,
        "evidence_schema": evidence.VERSION,
        "inputs": {"runs": {"path": str(runs_path), "sha256": digests["runs"]},
                   "native": {"path": str(native_path), "sha256": digests["native"]}},
        "gated_arm": GATED, "reference_arm": REFERENCE, "gated_seeds": GATED_SEEDS, "anchor_seed": ANCHOR_SEED,
        "claim_scope": "Counted attempts, rejections, error/rtol and matched-error work on a fixed deterministic "
                       "corpus of initial steps (not random samples); no wall-time or population claim; not a "
                       "default promotion.",
        "validity": {"status": "VALID", "rows": len(rows), "reference_limited_cells": derived["reference_limited"],
                     "reference_limited_count": len(derived["reference_limited"]),
                     "reference_limited_fraction": len(derived["reference_limited"]) / GATED_CELLS,
                     "gated_cells": GATED_CELLS, "parity_tests": derived["parity"],
                     "min_output_clipped_steps_successful": min(
                         (r["output_clipped_steps"] for r in rows if ok_success(r)), default=None)},
        "gates": gates,
        "verdict": verdict,
        "reported": {
            "cheapest_run_note": "cheapest-run ratios (C, fewest attempts among runs with error <= E) are reported, "
                                 "not gated",
            "ratios": ratios,
            "fit_diagnostics": diagnostics,
            "equal_rtol_error_ratios_gated": equal_rtol_distributions(gated_rows),
            "error_over_rtol_gated": error_over_rtol_summary(gated_rows),
            "cells_err_over_rtol_above_10": high_cells(index),
            "telemetry_totals": telemetry_totals(rows),
            "PREDcap2_vs_PREDcap": predcap_comparison(index),
            "endpoint_error_items_1_and_4": {"1_matched_error_work": endpoint_items["1_matched_error_work"],
                                             "4_relative_calibration": endpoint_items["4_relative_calibration"]},
            "gate_items_without_reference_limited_exclusion": unexcluded_items,
            "PREDcap_against_the_items": predcap_items,
            "rejection_counts_gated": rejections,
            "anchor_seed": anchor_report(index, set()),
            "references": references,
        },
    }
    output.write_text(json.dumps(out, indent=1, allow_nan=False) + "\n")
    print(f"verdict {verdict}")
    for name, g in gates.items():
        print(f"  {name}: {'pass' if g['pass'] else 'FAIL'}")
    return 0 if verdict == "PASS" else 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--native", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parent.parent,
                        help="where the telemetry parity tests are looked up (default: this checkout)")
    args = parser.parse_args()
    return run(args.runs, args.native, args.output, args.repo_root)


if __name__ == "__main__":
    sys.exit(main())
