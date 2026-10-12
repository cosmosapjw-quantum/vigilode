#!/usr/bin/env python3
"""Gate of research node research/as04_stage_residual_telemetry_20261011 (AS04; see its PREREGISTRATION.md).

Inputs: RUNS.json (`export_runs` of crates/rodas5p-integrators/tests/stage_residual_telemetry.rs; `.json.gz` is read
too), the re-audit NATIVE.json (S1 reference reports) and ALG06's RUNS.json (item 1b anchor). Both reference files
are immutable inputs bound to their committed SHA-256. Telemetry only: no record is an error certificate, a met
target is no forward-error statement, and nothing here is a wall-time claim.

Evidence (INVALID, exit 2) before any gate:
- AS03 first: tools/evidence_schema_v2.py primitives (strict JSON, the SHA-256 pins of NATIVE.json and ALG06
  RUNS.json, exact key sets, raw duplicate detection, typed numbers, canonical 16-digit binary64 hex);
- the registered case and run set: the 19 solver cells (S1 9, S2 4, S3 2, S4 1, S5 3) and the 9 driver runs (D1,
  D2, R x7), each driver run exactly once with telemetry on and once off, with the registered options, budget and
  rtol;
- provenance: the recorded source commit exists, is HEAD or an ancestor of it, holds the AS04 implementation and
  this checker (the checker blob at the source commit equals the checker being run, so the checker does not
  postdate the run), and every recorded tree status is clean.

Gate (PASS iff items 1-6 pass; everything else FAIL with every number preserved; exact, no tolerance):
1a. Telemetry on/off bitwise in every R and D run (all output times and states, attempts, accepted/rejected steps,
    state reuses, every WorkCounters field, StageSolveStatistics, the charge log); no pre-existing test file is
    modified (relative to the registration commit every changed path under crates/*/tests/ is an addition, and the
    `#[cfg(test)]` tail of every changed crates/*/src file is unchanged); the touched crates' test suites pass
    (`cargo test --offline --locked -p rodas5p-krylov -p rodas5p-integrators --all-targets`, run by this checker on
    the checked-out tree). Kill rule: a 1a failure means the telemetry is not adopted.
1b. The telemetry-off B3/B3b rows equal ALG06 RUNS's b3/b3b rows in ALG06's fields. A 1b failure is reported as
    anchor drift; the kill rule does not apply.
2.  Per solve arnoldi_jvps + residual_jvps = the solve's Krylov matvec delta (and, for the solver cells, the
    counters' linear_matvecs); per run sum(attempt jvp_calls) + jvp_outside_attempts = the run's jvp_calls;
    uncounted_shadow_matvecs = the wrapper's independent count in S3 and 0 wherever classification is off;
    acceptance_vector_copies = the documented copies of the recorded evaluations, whose number equals
    fallback_evaluations; the Copy assertion is in the source at the source commit.
3.  Every accepted record with actual > requested is BudgetExceeded; every attempt counts exactly those stages
    in budget_exceeded; S1: exactly triangular2_stall, triangular2_floor, triangular4_floor are BudgetExceeded with
    actual bits = NATIVE residual_norm, the other six Met; S4: Converged and BudgetExceeded.
4.  Records = StageSolveStatistics (solves and the five reason counts); one attempt record per attempt; requested
    <= raised and tightened <= raised; the reason implications; telemetry = report (solver cells); S1 reports =
    NATIVE reports bitwise; verified_bound Unavailable everywhere; the registered solver-cell outcomes.
5.  committed_debit = sum of debit_all over Committed attempts; debit, debit_all and debit_fallback_only recomputed
    in binary64 in stage order; work_all_attempts = the sum over every attempt, each attempt's work = its stage
    records' sums; D1 has a RejectedError or FailedLinear attempt with nonzero JVPs; debit_fallback_only = the
    charge-log charge bitwise for every charged attempt; debit_all >= debit_fallback_only in every attempt.
6.  D2: generation increments exactly at the re-linearizations (independently, a re-linearization evaluates
    f(t, y) once more: rhs_calls = 7 + relinearized in every complete attempt), model_epoch changes exactly at the
    fourth attempt, stage records carry their attempt's epoch; metric ScaledWrms in coupled runs, UnscaledL2 in
    ProjL2.

Interpretations and deviations, fixed before the recorded run (this checker is committed before RUNS.json exists):
- Requested targets: coupled policies eps_i sqrt(n) (config atol); other staged policies and every solver cell
  except S4 max(atol, rtol ||b||_2) of the solve's configuration (for S1 this is the recorded threshold, bit for
  bit); S4 atol.
- Copies table: the registered four branches are applied literally on evaluations that compared the residual norm
  (scaled-literal 1; scaled-homogeneous 2, 1 when r_le_atol; unscaled-literal 0; unscaled-homogeneous 1). The code
  makes 0 copies on unscaled-homogeneous with r_le_atol; that case would FAIL item 2 and is reached by no
  registered cell. The registration does not cover evaluations that exit before the comparison or meet an
  unrepresentable threshold; their documented copies (counted where the code copies) are: candidate rejected 0;
  residual rejected 1 scaled, 0 unscaled; unrepresentable threshold 1 scaled, 0 unscaled.
- S2 uses ProductionFallbackRule (AS01) in the fallback seam: physical rhs = scale x [1.5, 1.5, 0, 0], production
  rtol 1e-10 (overflowing 1e308, control 1e100) and the loose rules rtol 1.0 (finite scale 1, homogeneous scale
  1e308), atol 1e-14, scaled solver rtol 1e-14. S3's fixture fallback is the unscaled production rule with rtol 2
  (it accepts the abort as the fixture's accept-all closure does). The registered cell outcomes are gated under
  item 4: overflowing and control Failed (BudgetExhausted); loose cells FallbackAccepted and BudgetExceeded; S3
  FallbackAccepted after a contraction abort; S5 NonFinite, Breakdown and BudgetExhausted with no fallback call.
- NaN in NATIVE reports is the string "NaN"; it matches any NaN bits. ALG06 floats (JSON numbers) are compared as
  binary64 bits; the counters compared are ALG06's keys (the WorkCounters key set, COUNTERS_STAGED of AS03).
- ProjL2 + production fallback uses the ALG04 stage-target budget 200; B3/B3b use ALG06's 2,000.
- The per-attempt "BudgetExceeded stages" count is checked for every attempt (registered: committed ones).
- debit_fallback_only is the G3 charge of the attempt's staged solves (0 for non-coupled policies); a charged
  attempt is one with a fallback-accepted stage whose embedded error was computed (the ALG06 charge log rule).
- D2: the policy is CoupledGuarded2 (budget 200), y' = A y with A = [[-1, 1], [0, -1000]], y0 = [1, 1], T = 1,
  atol 1e-8, rtol 1e-6, h0 = 0.5; the epoch closure returns 1 for its first three reads and 2 afterwards (the
  driver reads it once per attempt; each twin has its own closure).
- Reported, never gated: per-reason and per-attainment counts, actual/requested quantiles, committed_debit against
  the G3 charge sum, attempts whose error norm + debit_all would cross 1, telemetry memory, per-solve JVP deltas,
  jvp_outside_attempts, and the predictions.
- HOLD: if a completeness item (3-4) fails, the reviewer decides whether completeness would need a decision change
  (then the ledger records FAIL with claim "decision HOLD"); the checker reports the item failures only.
The output file is immutable.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import math
import re
import struct
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

SCHEMA = "vigilode-as04-stage-telemetry-check-v1"
RUNS_SCHEMA = "vigilode-as04-runs-v1"
NODE = "as04_stage_residual_telemetry_20261011"
REGISTRATION_COMMIT = "8ce9bda"
NATIVE_SHA256 = "b9404714af837dfb099db564550e88127ff52a0c9341758862529261b85bd364"
ALG06_RUNS_SHA256 = "26aaa444f06149335e904558805797067c0fdcc53c976b92ea1ee3fe56e67432"
CHECKER_PATH = "tools/as04_stage_telemetry_check.py"
IMPLEMENTATION_MARKERS = {
    "crates/rodas5p-krylov/src/gmres_staged.rs": ("pub fn solve_staged_gmres_observed",),
    "crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs": (
        "pub fn set_stage_telemetry", "pub fn integrate_rodas5p_mf_fast_observed_with_stage_telemetry",
        "assert_copy::<StageResidualRecord>();", "assert_copy::<AttemptResidualRecord>();",
        "assert_copy::<StagedSolveTelemetry>();"),
    "crates/rodas5p-integrators/tests/stage_residual_telemetry.rs": ("fn export_runs()",),
    "crates/rodas5p-krylov/tests/staged_solve_telemetry.rs": ("solve_staged_gmres_observed",),
    "tools/test_as04_stage_telemetry_check.py": ("as04_stage_telemetry_check",),
}
TEST_SUITE_COMMAND = ["cargo", "test", "--offline", "--locked", "-p", "rodas5p-krylov", "-p", "rodas5p-integrators",
                      "--all-targets"]
STAGES = 8
HEX64 = re.compile(r"^[0-9a-f]{16}$")
SHA1 = re.compile(r"^[0-9a-f]{40}$")

# ------------------------------------------------------------------------------------------- registered
S1_IDS = tuple(f"{s}_{m}" for s in ("identity2", "triangular2", "triangular4") for m in ("strict", "stall", "floor"))
S1_EXCEEDED = ("triangular2_stall", "triangular2_floor", "triangular4_floor")
SOLVER_CELLS = ([("S1", i) for i in S1_IDS]
                + [("S2", i) for i in ("overflowing_norms", "finite_norm_control", "loose_finite",
                                       "loose_homogeneous")]
                + [("S3", "contraction_unclassified"), ("S3", "contraction_classified"), ("S4", "raised_target")]
                + [("S5", i) for i in ("nonfinite", "breakdown", "budget_exhausted")])


def _options(policy: str, guard: bool, fallback: bool, classify: bool, g1: bool, g2: bool, g3: bool) -> dict:
    return {"policy": policy, "stagnation_guard": guard, "production_fallback": fallback,
            "classify_guard_aborts": classify, "classify_accepted_guard_aborts": classify,
            "effective_cycle_overrun": g1, "floor_at_confirmations": g2, "charge_fallback_residual": g3}


B3 = _options("CoupledGuarded2", True, True, True, True, True, True)
B3B = _options("CoupledGuarded2", True, True, True, True, True, False)
PROJ = _options("ProjL2", False, True, False, False, False, False)
D1_OPTIONS = _options("Coupled", False, True, False, False, False, False)
D2_OPTIONS = _options("CoupledGuarded2", False, True, False, False, False, False)
# (cell, id, arm) -> (options, budget, rtol, dimension)
DRIVER_RUNS = {
    ("D1", "as01-cyclic-4-overflow", "Coupled+fallback"): (D1_OPTIONS, 1, 1e-06, 4),
    ("D2", "as04-d2-linear-2", "CoupledGuarded2"): (D2_OPTIONS, 200, 1e-06, 2),
    ("R", "robertson-4e10/1e-5", "B3"): (B3, 2000, 1e-05, 3),
    ("R", "robertson-4e10/1e-9", "B3"): (B3, 2000, 1e-09, 3),
    ("R", "stosc-w1e4/1e-4", "B3"): (B3, 2000, 1e-04, 128),
    ("R", "van-der-pol-mu1000/1e-6", "B3"): (B3, 2000, 1e-06, 2),
    ("R", "brusselator-1d-50/1e-6", "B3"): (B3, 2000, 1e-06, 100),
    ("R", "robertson-4e10/1e-5", "B3b"): (B3B, 2000, 1e-05, 3),
    ("R", "brusselator-1d-50/1e-6", "ProjL2+fallback"): (PROJ, 200, 1e-06, 100),
}
# (id, arm) -> (ALG06 case, rtol, ALG06 arm)
ANCHORS = {
    ("robertson-4e10/1e-5", "B3"): ("robertson-4e10", 1e-05, "b3"),
    ("robertson-4e10/1e-9", "B3"): ("robertson-4e10", 1e-09, "b3"),
    ("stosc-w1e4/1e-4", "B3"): ("stosc-w1e4", 1e-04, "b3"),
    ("van-der-pol-mu1000/1e-6", "B3"): ("van-der-pol-mu1000", 1e-06, "b3"),
    ("brusselator-1d-50/1e-6", "B3"): ("brusselator-1d-50", 1e-06, "b3"),
    ("robertson-4e10/1e-5", "B3b"): ("robertson-4e10", 1e-05, "b3b"),
}
COUPLED_POLICIES = frozenset(("Coupled", "CoupledGuarded", "CoupledGuarded2"))
REASONS = ("Converged", "StallAccepted", "FloorAccepted", "FallbackAccepted", "Failed")
REASON_STAT = {"Converged": "converged", "StallAccepted": "stall_accepted", "FloorAccepted": "floor_accepted",
               "FallbackAccepted": "fallback_accepted", "Failed": "failed"}
REASON_ATTEMPT = REASON_STAT
ATTAINMENTS = ("Met", "BudgetExceeded", "Unverified")
OUTCOMES = ("Committed", "RejectedError", "FailedLinear", "FailedOther")
BRANCHES = ("ScaledLiteral", "ScaledHomogeneous", "ScaledUnrepresentable", "UnscaledLiteral",
            "UnscaledHomogeneous", "UnscaledUnrepresentable")
EXITS = ("CandidateRejected", "ResidualRejected", "Compared")
GUARD_ABORTS = ("Contraction", "Overrun")
FAILURES = ("BudgetExhausted", "GuardContraction", "GuardOverrun", "NonFinite", "Breakdown")

# ------------------------------------------------------------------------------------------- key sets
DOC_KEYS = frozenset(("schema", "node", "export", "provenance", "stages_per_attempt", "solver_cells",
                      "driver_runs"))
PROVENANCE_KEYS = frozenset(("source_commit", "tree_status_before", "tree_status_after", "rayon_num_threads",
                             "openblas_num_threads"))
CELL_KEYS = frozenset(("cell", "id", "dimension", "config", "report", "telemetry", "evaluations", "output",
                       "counters", "independent_shadow_matvecs"))
CELL_CONFIG_KEYS = frozenset(("restart", "max_columns", "rtol", "atol", "stall_rule", "floor_at_confirmations",
                              "stagnation_guard", "classify_guard_aborts", "classify_accepted_guard_aborts"))
REPORT_KEYS = frozenset(("outcome", "failure", "accepted", "threshold", "final_threshold", "residual_norm",
                         "residual_is_finite", "projected_residual", "right_norm", "columns", "cycles",
                         "true_residuals", "confirmations", "failed_confirmations", "budget_exhausted",
                         "guard_abort", "matvecs", "nu_max", "nu_flops", "nu_evaluations", "shadow_converged",
                         "exhaustion"))
REPORT_HEX = ("threshold", "final_threshold", "residual_norm", "projected_residual", "right_norm", "nu_max")
REPORT_INT = ("columns", "cycles", "true_residuals", "confirmations", "failed_confirmations", "matvecs", "nu_flops",
              "nu_evaluations")
SOLVER_KEYS = frozenset(("reason", "requested_target", "raised_target", "tightened_target", "attainable_floor",
                         "actual_residual", "actual_residual_inf", "attainment", "heuristic_gain", "verified_bound",
                         "arnoldi_jvps", "residual_jvps", "fallback_evaluations", "acceptance_vector_copies",
                         "nu_flops", "uncounted_shadow_matvecs", "columns", "matvecs", "guard_abort",
                         "budget_exhausted"))
SOLVER_HEX = ("requested_target", "raised_target", "tightened_target", "attainable_floor", "actual_residual",
              "actual_residual_inf")
SOLVER_INT = ("arnoldi_jvps", "residual_jvps", "fallback_evaluations", "acceptance_vector_copies", "nu_flops",
              "uncounted_shadow_matvecs", "columns", "matvecs")
GAIN_KEYS = frozenset(("label", "nu_max", "nu_evaluations"))
EVALUATION_KEYS = frozenset(("branch", "exit", "r_le_atol", "vector_copies", "accepted"))
RUN_KEYS = frozenset(("cell", "id", "arm", "telemetry", "dimension", "budget", "options", "rtol", "result",
                      "records"))
RESULT_KEYS = frozenset(("ok", "success", "message", "t", "y", "attempts", "accepted", "rejected", "state_reuses",
                         "internal_steps", "output_clipped_steps", "counters", "stage_statistics", "charges"))
RESULT_ERROR_KEYS = frozenset(("ok", "error"))
RESULT_INT = ("attempts", "accepted", "rejected", "state_reuses", "internal_steps", "output_clipped_steps")
STAT_FLOAT = ("charge_max", "charge_sum", "nu_max")
RECORDS_KEYS = frozenset(("stages", "attempts", "committed_debit", "work_all_attempts", "jvp_outside_attempts",
                          "memory_bytes"))
STAGE_KEYS = frozenset(("attempt", "stage", "solver", "metric", "operator_epoch", "h", "gamma", "tau_e", "debit",
                        "fallback", "jvp_calls"))
METRIC_WRMS_KEYS = frozenset(("kind", "atol", "rtol", "state_digest"))
EPOCH_KEYS = frozenset(("model_epoch", "generation"))
WORK_KEYS = ("jvp_calls", "linear_matvecs", "rhs_calls", "krylov_columns", "arnoldi_jvps", "residual_jvps",
             "shadow_matvecs", "vector_copies")
ATTEMPT_KEYS = frozenset(("attempt", "t", "h", "outcome", "operator_epoch", "relinearized", "stages", "converged",
                          "stall_accepted", "floor_accepted", "fallback_accepted", "failed", "budget_exceeded",
                          "max_actual_over_requested", "debit_all", "debit_fallback_only", "error_norm", "work"))
ATTEMPT_INT = ("stages", "converged", "stall_accepted", "floor_accepted", "fallback_accepted", "failed",
               "budget_exceeded")
ATTEMPT_HEX = ("t", "h", "max_actual_over_requested", "debit_all", "debit_fallback_only", "error_norm")
NATIVE_REPORT_KEYS = ("accepted", "budget_exhausted", "columns", "confirmations", "cycles", "failed_confirmations",
                      "failure", "final_threshold", "guard_abort", "matvecs", "nu_flops", "nu_max", "outcome",
                      "projected_residual", "residual_is_finite", "residual_norm", "right_norm", "threshold",
                      "true_residuals")
ALG06_COMPARED = ("ok", "success", "message", "t", "attempts", "accepted", "rejected", "state_reuses",
                  "internal_steps", "output_clipped_steps")


# ------------------------------------------------------------------------------------------- numbers
def f(h: str) -> float:
    """A binary64 from its 16-digit big-endian hex (non-finite allowed)."""
    return struct.unpack(">d", bytes.fromhex(h))[0]


def hx(x: float) -> str:
    return struct.pack(">d", x).hex()


def div(a: float, b: float) -> float:
    """IEEE binary64 division (Python raises on a zero divisor)."""
    if b != 0.0 or math.isnan(b):
        return a / b
    if a == 0.0 or math.isnan(a):
        return math.nan
    return math.copysign(math.inf, a) * math.copysign(1.0, b)


def same(h: str, x: float) -> bool:
    """The hex binary64 `h` is `x` bit for bit, or both are NaN (NaN payload and sign are not compared)."""
    return (math.isnan(x) and math.isnan(f(h))) or h == hx(x)


def classify(actual: float, requested: float) -> str:
    if not (math.isfinite(actual) and math.isfinite(requested)):
        return "Unverified"
    return "Met" if actual <= requested else "BudgetExceeded"


def documented_copies(e: dict) -> int:
    """The registered copies table (literal) plus the disclosed exits; see the module docstring."""
    scaled = e["branch"].startswith("Scaled")
    if e["exit"] == "CandidateRejected":
        return 0
    if e["exit"] == "ResidualRejected":
        return 1 if scaled else 0
    branch = e["branch"]
    if branch == "ScaledLiteral":
        return 1
    if branch == "ScaledHomogeneous":
        return 1 if e["r_le_atol"] is True else 2
    if branch == "UnscaledLiteral":
        return 0
    if branch == "UnscaledHomogeneous":
        return 1  # registered literally (the code makes 0 when r_le_atol; disclosed)
    return 1 if scaled else 0  # unrepresentable threshold


# ------------------------------------------------------------------------------------------- evidence
def _hex(c, path: str, value: Any) -> bool:
    if not isinstance(value, str) or not HEX64.match(value):
        c.fail(path, f"not a 16-digit lowercase binary64 hex string: {value!r}")
        return False
    return True


def _enum(c, path: str, value: Any, allowed, nullable: bool = False) -> None:
    if value is None and nullable:
        return
    if value not in allowed:
        c.fail(path, f"{value!r} not in {list(allowed)}")


def _solver(c, path: str, t: Any) -> None:
    if not c.keys(path, t, SOLVER_KEYS):
        return
    _enum(c, f"{path}.reason", t["reason"], REASONS)
    _enum(c, f"{path}.attainment", t["attainment"], ATTAINMENTS)
    _enum(c, f"{path}.guard_abort", t["guard_abort"], GUARD_ABORTS, nullable=True)
    c.str_(f"{path}.verified_bound", t["verified_bound"])
    c.bool_(f"{path}.budget_exhausted", t["budget_exhausted"])
    for k in SOLVER_HEX:
        _hex(c, f"{path}.{k}", t[k])
    for k in SOLVER_INT:
        c.int_(f"{path}.{k}", t[k])
    g = t["heuristic_gain"]
    if c.keys(f"{path}.heuristic_gain", g, GAIN_KEYS):
        c.equal(f"{path}.heuristic_gain.label", g["label"], "heuristic")
        _hex(c, f"{path}.heuristic_gain.nu_max", g["nu_max"])
        c.int_(f"{path}.heuristic_gain.nu_evaluations", g["nu_evaluations"])


def _evaluation(c, path: str, e: Any) -> None:
    if not c.keys(path, e, EVALUATION_KEYS):
        return
    _enum(c, f"{path}.branch", e["branch"], BRANCHES)
    _enum(c, f"{path}.exit", e["exit"], EXITS)
    if e["r_le_atol"] is not None:
        c.bool_(f"{path}.r_le_atol", e["r_le_atol"])
    c.int_(f"{path}.vector_copies", e["vector_copies"])
    c.bool_(f"{path}.accepted", e["accepted"])


def _report(c, path: str, r: Any) -> None:
    if not c.keys(path, r, REPORT_KEYS):
        return
    _enum(c, f"{path}.outcome", r["outcome"], REASONS)
    _enum(c, f"{path}.failure", r["failure"], FAILURES, nullable=True)
    _enum(c, f"{path}.guard_abort", r["guard_abort"], GUARD_ABORTS, nullable=True)
    for k in ("accepted", "residual_is_finite", "budget_exhausted", "exhaustion"):
        c.bool_(f"{path}.{k}", r[k])
    if r["shadow_converged"] is not None:
        c.bool_(f"{path}.shadow_converged", r["shadow_converged"])
    for k in REPORT_HEX:
        _hex(c, f"{path}.{k}", r[k])
    for k in REPORT_INT:
        c.int_(f"{path}.{k}", r[k])


def _work(c, path: str, w: Any) -> None:
    if c.keys(path, w, WORK_KEYS):
        for k in WORK_KEYS:
            c.int_(f"{path}.{k}", w[k])


def _epoch(c, path: str, e: Any) -> None:
    if c.keys(path, e, EPOCH_KEYS):
        if e["model_epoch"] is not None:
            c.int_(f"{path}.model_epoch", e["model_epoch"])
        c.int_(f"{path}.generation", e["generation"])


def _result(c, path: str, r: Any, dimension: int) -> None:
    if not c.dict_(path, r) or not c.bool_(f"{path}.ok", r.get("ok")):
        return
    if not r["ok"]:
        if c.keys(path, r, RESULT_ERROR_KEYS):
            c.str_(f"{path}.error", r["error"])
        return
    if not c.keys(path, r, RESULT_KEYS):
        return
    c.bool_(f"{path}.success", r["success"])
    c.str_(f"{path}.message", r["message"], nonempty=False)
    for k in RESULT_INT:
        c.int_(f"{path}.{k}", r[k])
    c.hex_vector(f"{path}.t", r["t"])
    if c.list_(f"{path}.y", r["y"], nonempty=True):
        if not isinstance(r["t"], list) or len(r["y"]) != len(r["t"]):
            c.fail(f"{path}.y", "one state per output time required")
        for i, y in enumerate(r["y"]):
            c.hex_vector(f"{path}.y[{i}]", y, dimension)
    c.counters(f"{path}.counters", r["counters"], evidence.COUNTERS_STAGED)
    s = r["stage_statistics"]
    if c.keys(f"{path}.stage_statistics", s, evidence.STAGE_STATISTICS_INT | evidence.STAGE_STATISTICS_FLOAT):
        for k in sorted(s):
            if k in evidence.STAGE_STATISTICS_FLOAT:
                _hex(c, f"{path}.stage_statistics.{k}", s[k])
            else:
                c.int_(f"{path}.stage_statistics.{k}", s[k])
    if c.list_(f"{path}.charges", r["charges"]):
        for i, row in enumerate(r["charges"]):
            p = f"{path}.charges[{i}]"
            if not isinstance(row, list) or len(row) != 6:
                c.fail(p, "charge row must be [t, h, error, charge, stages, charged]")
                continue
            for j in range(4):
                _hex(c, f"{p}[{j}]", row[j])
            c.int_(f"{p}.stages", row[4])
            c.bool_(f"{p}.charged", row[5])


def _records(c, path: str, rec: Any) -> None:
    if not c.keys(path, rec, RECORDS_KEYS):
        return
    _hex(c, f"{path}.committed_debit", rec["committed_debit"])
    _work(c, f"{path}.work_all_attempts", rec["work_all_attempts"])
    c.int_(f"{path}.jvp_outside_attempts", rec["jvp_outside_attempts"])
    c.int_(f"{path}.memory_bytes", rec["memory_bytes"])
    if c.list_(f"{path}.stages", rec["stages"]):
        for i, s in enumerate(rec["stages"]):
            p = f"{path}.stages[{i}]"
            if not c.keys(p, s, STAGE_KEYS):
                continue
            c.int_(f"{p}.attempt", s["attempt"])
            c.int_(f"{p}.stage", s["stage"])
            c.int_(f"{p}.jvp_calls", s["jvp_calls"])
            _solver(c, f"{p}.solver", s["solver"])
            for k in ("h", "gamma", "tau_e", "debit"):
                _hex(c, f"{p}.{k}", s[k])
            _epoch(c, f"{p}.operator_epoch", s["operator_epoch"])
            m = s["metric"]
            if c.dict_(f"{p}.metric", m):
                if m.get("kind") == "ScaledWrms":
                    if c.keys(f"{p}.metric", m, METRIC_WRMS_KEYS):
                        for k in ("atol", "rtol", "state_digest"):
                            _hex(c, f"{p}.metric.{k}", m[k])
                elif m != {"kind": "UnscaledL2"}:
                    c.fail(f"{p}.metric", f"unregistered metric {m!r}")
            if s["fallback"] is not None:
                _evaluation(c, f"{p}.fallback", s["fallback"])
    if c.list_(f"{path}.attempts", rec["attempts"]):
        for i, a in enumerate(rec["attempts"]):
            p = f"{path}.attempts[{i}]"
            if not c.keys(p, a, ATTEMPT_KEYS):
                continue
            c.int_(f"{p}.attempt", a["attempt"])
            _enum(c, f"{p}.outcome", a["outcome"], OUTCOMES)
            c.bool_(f"{p}.relinearized", a["relinearized"])
            for k in ATTEMPT_INT:
                c.int_(f"{p}.{k}", a[k])
            for k in ATTEMPT_HEX:
                _hex(c, f"{p}.{k}", a[k])
            _epoch(c, f"{p}.operator_epoch", a["operator_epoch"])
            _work(c, f"{p}.work", a["work"])


def validate(runs: Any) -> tuple[dict, dict]:
    """Type RUNS against the registered schema and case/run set; return the indexed cells and runs."""
    c = evidence._Checks()
    if not c.keys("runs", runs, DOC_KEYS):
        c.done()
    c.equal("runs.schema", runs["schema"], RUNS_SCHEMA)
    c.equal("runs.node", runs["node"], NODE)
    c.equal("runs.export", runs["export"], "export_runs")
    c.equal("runs.stages_per_attempt", runs["stages_per_attempt"], STAGES)
    prov = runs["provenance"]
    if c.keys("runs.provenance", prov, PROVENANCE_KEYS):
        if not (isinstance(prov["source_commit"], str) and SHA1.match(prov["source_commit"])):
            c.fail("runs.provenance.source_commit", f"not a 40-digit commit: {prov['source_commit']!r}")
        for k in ("tree_status_before", "tree_status_after"):
            c.str_(f"runs.provenance.{k}", prov[k], nonempty=False)
        for k in ("rayon_num_threads", "openblas_num_threads"):
            c.equal(f"runs.provenance.{k}", prov[k], "1")
    cells: dict = {}
    if c.list_("runs.solver_cells", runs["solver_cells"], nonempty=True):
        good = []
        for i, cell in enumerate(runs["solver_cells"]):
            p = f"runs.solver_cells[{i}]"
            if not c.keys(p, cell, CELL_KEYS):
                continue
            c.str_(f"{p}.cell", cell["cell"])
            c.str_(f"{p}.id", cell["id"])
            c.int_(f"{p}.dimension", cell["dimension"], minimum=1)
            cfg = cell["config"]
            if c.keys(f"{p}.config", cfg, CELL_CONFIG_KEYS):
                for k in ("restart", "max_columns"):
                    c.int_(f"{p}.config.{k}", cfg[k], minimum=1)
                for k in ("rtol", "atol"):
                    _hex(c, f"{p}.config.{k}", cfg[k])
                for k in sorted(CELL_CONFIG_KEYS - {"restart", "max_columns", "rtol", "atol"}):
                    c.bool_(f"{p}.config.{k}", cfg[k])
            _report(c, f"{p}.report", cell["report"])
            _solver(c, f"{p}.telemetry", cell["telemetry"])
            if c.list_(f"{p}.evaluations", cell["evaluations"]):
                for j, e in enumerate(cell["evaluations"]):
                    _evaluation(c, f"{p}.evaluations[{j}]", e)
            if c.list_(f"{p}.output", cell["output"], nonempty=True):
                for j, v in enumerate(cell["output"]):
                    _hex(c, f"{p}.output[{j}]", v)
            c.counters(f"{p}.counters", cell["counters"], evidence.COUNTERS_STAGED)
            if cell["independent_shadow_matvecs"] is not None:
                c.int_(f"{p}.independent_shadow_matvecs", cell["independent_shadow_matvecs"])
            elif cell["cell"] == "S3":
                c.fail(f"{p}.independent_shadow_matvecs", "S3 needs the independent count")
            good.append(cell)
        cells = c.unique_index("runs.solver_cells", good, lambda x: (x["cell"], x["id"]))
        if sorted(cells) != sorted(SOLVER_CELLS):
            c.fail("runs.solver_cells", f"cell set {sorted(cells)} != registered {sorted(SOLVER_CELLS)}")
    driver: dict = {}
    if c.list_("runs.driver_runs", runs["driver_runs"], nonempty=True):
        good = []
        for i, run in enumerate(runs["driver_runs"]):
            p = f"runs.driver_runs[{i}]"
            if not c.keys(p, run, RUN_KEYS):
                continue
            key = (run["cell"], run["id"], run["arm"])
            registered = DRIVER_RUNS.get(key) if all(isinstance(x, str) for x in key) else None
            if registered is None:
                c.fail(p, f"unregistered driver run {key!r}")
                continue
            options, budget, rtol, dimension = registered
            c.bool_(f"{p}.telemetry", run["telemetry"])
            c.equal(f"{p}.options", run["options"], options)
            c.equal(f"{p}.budget", run["budget"], budget)
            c.equal(f"{p}.rtol", run["rtol"], rtol)
            c.equal(f"{p}.dimension", run["dimension"], dimension)
            _result(c, f"{p}.result", run["result"], dimension)
            if run["telemetry"] is True:
                if run["records"] is not None:
                    _records(c, f"{p}.records", run["records"])
                elif isinstance(run["result"], dict) and run["result"].get("ok") is True:
                    c.fail(f"{p}.records", "a successful telemetry-on run must carry its records")
            elif run["records"] is not None:
                c.fail(f"{p}.records", "a telemetry-off twin carries no records")
            good.append(run)
        driver = c.unique_index("runs.driver_runs", good,
                                lambda x: (x["cell"], x["id"], x["arm"], x["telemetry"]))
        expected = sorted((*k, t) for k in DRIVER_RUNS for t in (True, False))
        if sorted(driver) != expected:
            missing = sorted(set(expected) - set(driver))
            c.fail("runs.driver_runs", f"run set differs from the registered on/off twins (missing {missing})")
    c.done()
    return cells, driver


def validate_references(native: Any, alg06: Any) -> None:
    c = evidence._Checks()
    try:
        stage = native["implementation_adversaries"]["stage_acceptance"]
        ids = [s["id"] for s in stage]
        if sorted(ids) != sorted(S1_IDS) or len(set(ids)) != len(ids):
            c.fail("native.stage_acceptance", f"ids {ids} != registered S1")
        for s in stage:
            c.keys(f"native.stage_acceptance[{s['id']}].report", s["observed"]["report"], NATIVE_REPORT_KEYS)
    except (KeyError, TypeError) as exc:
        c.fail("native", f"malformed stage_acceptance: {exc!r}")
    try:
        rows = alg06["rows"]
        for (case, rtol, arm) in ANCHORS.values():
            hits = [r for r in rows if r["case"] == case and r["rtol"] == rtol]
            if len(hits) != 1:
                c.fail("alg06", f"{len(hits)} rows for {case} {rtol}")
                continue
            c.adaptive(f"alg06[{case} {rtol}].{arm}", hits[0]["arms"][arm], "staged", None)
    except (KeyError, TypeError) as exc:
        c.fail("alg06", f"malformed rows: {exc!r}")
    c.done()


# ------------------------------------------------------------------------------------------- provenance
def git(*args: str) -> str:
    out = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        raise evidence.ValidationError([f"git {' '.join(args)} failed: {out.stderr.strip()}"])
    return out.stdout


def git_blob(commit: str, path: str) -> str | None:
    out = subprocess.run(["git", "rev-parse", f"{commit}:{path}"], cwd=ROOT, capture_output=True, text=True)
    return out.stdout.strip() if out.returncode == 0 else None


def check_provenance(runs: dict, checker_file: Path) -> dict:
    """INVALID unless the RUNS source commit is the committed implementation with this checker, on a clean tree."""
    prov = runs["provenance"]
    commit = prov["source_commit"]
    reasons = []
    for k in ("tree_status_before", "tree_status_after"):
        if prov[k] != "":
            reasons.append(f"recorded {k} is dirty: {prov[k]!r}")
    if subprocess.run(["git", "cat-file", "-e", f"{commit}^{{commit}}"], cwd=ROOT).returncode != 0:
        raise evidence.ValidationError(reasons + [f"source commit {commit} is not in the repository"])
    head = git("rev-parse", "HEAD").strip()
    if subprocess.run(["git", "merge-base", "--is-ancestor", commit, head], cwd=ROOT).returncode != 0:
        reasons.append(f"source commit {commit} is not HEAD {head} or an ancestor of it")
    running = git("hash-object", str(checker_file)).strip()
    at_source = git_blob(commit, CHECKER_PATH)
    if at_source is None:
        reasons.append("the checker does not exist at the source commit (it postdates the run)")
    elif at_source != running:
        reasons.append(f"checker blob at the source commit {at_source} != the checker run {running}")
    for path, markers in IMPLEMENTATION_MARKERS.items():
        out = subprocess.run(["git", "show", f"{commit}:{path}"], cwd=ROOT, capture_output=True, text=True)
        if out.returncode != 0:
            reasons.append(f"{path} is missing at the source commit")
            continue
        for marker in markers:
            if marker not in out.stdout:
                reasons.append(f"{path} at the source commit lacks {marker!r}")
    if reasons:
        raise evidence.ValidationError(reasons)
    return {"source_commit": commit, "head": head, "checker_blob": running, "tree_clean": True}


def audit_test_files(commit: str) -> dict:
    """No pre-existing test file modified between the registration commit and the source commit."""
    out = git("diff", "--no-renames", "--name-status", REGISTRATION_COMMIT, commit)
    modified, added, tails = [], [], []
    for line in out.splitlines():
        status, path = line.split("\t", 1)
        if re.match(r"^crates/[^/]+/tests/", path):
            (added if status == "A" else modified).append(f"{status} {path}")
        elif re.match(r"^crates/[^/]+/src/.*\.rs$", path) and status == "M":
            before = git("show", f"{REGISTRATION_COMMIT}:{path}")
            after = git("show", f"{commit}:{path}")
            tail = lambda s: s[s.index("#[cfg(test)]"):] if "#[cfg(test)]" in s else ""  # noqa: E731
            if tail(before) != tail(after):
                tails.append(path)
    return {"added_test_files": added, "modified_test_files": modified, "changed_test_tails": tails,
            "ok": not modified and not tails}


def run_test_suites() -> dict:
    out = subprocess.run(TEST_SUITE_COMMAND, cwd=ROOT, capture_output=True, text=True)
    summary = [line for line in out.stdout.splitlines() if line.startswith("test result:")]
    passed = sum(int(m.group(1)) for line in summary if (m := re.search(r"(\d+) passed", line)))
    failed = sum(int(m.group(1)) for line in summary if (m := re.search(r"(\d+) failed", line)))
    return {"command": " ".join(TEST_SUITE_COMMAND), "exit_code": out.returncode, "binaries": len(summary),
            "passed": passed, "failed": failed, "ok": out.returncode == 0 and failed == 0 and len(summary) > 0}


# ------------------------------------------------------------------------------------------- gate items
class Item:
    def __init__(self, name: str):
        self.name = name
        self.failures: list[str] = []
        self.numbers: dict = {}

    def check(self, ok: bool, message: str) -> bool:
        if not ok:
            self.failures.append(message)
        return ok

    def value(self) -> dict:
        return {"pass": not self.failures, "failures": self.failures[:200], "failure_count": len(self.failures),
                "numbers": self.numbers}


def _solves(cells: dict, driver: dict):
    """Every solve record: (label, solver telemetry, evaluations list, run options or None)."""
    for (cell, cid), x in sorted(cells.items()):
        yield f"{cell}/{cid}", x["telemetry"], x["evaluations"], x
    for key, run in sorted(driver.items()):
        if run["telemetry"] and run["records"] is not None:
            for i, s in enumerate(run["records"]["stages"]):
                ev = [] if s["fallback"] is None else [s["fallback"]]
                yield f"{key[0]}/{key[1]}/{key[2]}#stage{i}", s["solver"], ev, None


def on_runs(driver: dict):
    for (cell, rid, arm, telemetry), run in sorted(driver.items()):
        if telemetry:
            yield (cell, rid, arm), run


def item1a(driver: dict, audit: dict | None, suites: dict | None) -> Item:
    it = Item("1a")
    equal = 0
    for key in sorted(DRIVER_RUNS):
        on, off = driver[(*key, True)], driver[(*key, False)]
        if it.check(on["result"] == off["result"], f"{key}: telemetry on and off results differ"):
            equal += 1
    it.numbers["twins_identical"] = f"{equal}/{len(DRIVER_RUNS)}"
    it.numbers["test_file_audit"] = audit
    it.check(audit is not None and audit["ok"], f"pre-existing test files modified or not audited: {audit}")
    it.numbers["test_suites"] = suites
    it.check(suites is not None and suites["ok"], f"pre-existing test suites not passing or not run: {suites}")
    return it


def item1b(driver: dict, alg06: dict) -> Item:
    it = Item("1b")
    rows = alg06["rows"]
    for (rid, arm), (case, rtol, alg_arm) in sorted(ANCHORS.items()):
        off = driver[("R", rid, arm, False)]["result"]
        ref = [r for r in rows if r["case"] == case and r["rtol"] == rtol][0]["arms"][alg_arm]
        label = f"{rid} {arm}"
        if not it.check(off.get("ok") is True, f"{label}: telemetry-off run failed"):
            continue
        for k in ALG06_COMPARED:
            it.check(off[k] == ref[k], f"{label}: {k} {off[k]!r} != ALG06 {ref[k]!r}")
        it.check(off["y"][-1] == ref["y_last"], f"{label}: final state differs from ALG06")
        it.check(set(off["counters"]) == set(ref["counters"]), f"{label}: counter key sets differ from ALG06")
        for k, v in ref["counters"].items():
            it.check(off["counters"].get(k) == v, f"{label}: counter {k} {off['counters'].get(k)} != ALG06 {v}")
        extra = sorted(set(off["counters"]) - set(ref["counters"]))
        it.numbers.setdefault("counters_not_in_alg06", {})[label] = {k: off["counters"][k] for k in extra}
        for k, v in ref["stage_statistics"].items():
            mine = off["stage_statistics"].get(k)
            if k in STAT_FLOAT:
                it.check(mine == hx(v), f"{label}: stage_statistics.{k} {mine} != ALG06 {hx(v)}")
            else:
                it.check(mine == v, f"{label}: stage_statistics.{k} {mine} != ALG06 {v}")
        ok = len(off["charges"]) == len(ref["charges"])
        it.check(ok, f"{label}: {len(off['charges'])} charges != ALG06 {len(ref['charges'])}")
        if ok:
            for i, (a, b) in enumerate(zip(off["charges"], ref["charges"])):
                it.check(a == [hx(b[0]), hx(b[1]), hx(b[2]), hx(b[3]), b[4], b[5]],
                         f"{label}: charge row {i} differs from ALG06")
    it.numbers["anchors"] = len(ANCHORS)
    return it


def item2(cells: dict, driver: dict) -> Item:
    it = Item("2")
    solves = 0
    for label, t, evs, cell in _solves(cells, driver):
        solves += 1
        it.check(t["arnoldi_jvps"] + t["residual_jvps"] == t["matvecs"],
                 f"{label}: arnoldi {t['arnoldi_jvps']} + residual {t['residual_jvps']} != matvecs {t['matvecs']}")
        if cell is not None:
            it.check(cell["counters"]["linear_matvecs"] == t["matvecs"],
                     f"{label}: counters.linear_matvecs != matvecs")
        it.check(len(evs) == t["fallback_evaluations"],
                 f"{label}: {len(evs)} recorded evaluations != fallback_evaluations {t['fallback_evaluations']}")
        documented = sum(documented_copies(e) for e in evs)
        it.check(t["acceptance_vector_copies"] == documented,
                 f"{label}: acceptance_vector_copies {t['acceptance_vector_copies']} != documented {documented}")
        for e in evs:
            it.check(e["vector_copies"] == documented_copies(e), f"{label}: evaluation {e} copies undocumented")
    it.numbers["solves_checked"] = solves
    # Shadow applications.
    for (cell, cid), x in sorted(cells.items()):
        t = x["telemetry"]
        if cell == "S3":
            it.check(t["uncounted_shadow_matvecs"] == x["independent_shadow_matvecs"],
                     f"S3/{cid}: uncounted {t['uncounted_shadow_matvecs']} != independent "
                     f"{x['independent_shadow_matvecs']}")
            it.numbers[f"S3/{cid}/shadow_matvecs"] = t["uncounted_shadow_matvecs"]
        if not x["config"]["classify_guard_aborts"]:
            it.check(t["uncounted_shadow_matvecs"] == 0, f"{cell}/{cid}: shadow work without classification")
    it.check(cells[("S3", "contraction_classified")]["independent_shadow_matvecs"] > 0,
             "S3: the classified abort ran no shadow continuation")
    jvp = {}
    for key, run in on_runs(driver):
        rec = run["records"]
        if rec is None:
            it.check(False, f"{key}: no records")
            continue
        if not run["options"]["classify_guard_aborts"]:
            n = sum(s["solver"]["uncounted_shadow_matvecs"] for s in rec["stages"])
            it.check(n == 0, f"{key}: {n} shadow matvecs without classification")
        total = sum(a["work"]["jvp_calls"] for a in rec["attempts"]) + rec["jvp_outside_attempts"]
        run_jvp = run["result"]["counters"]["jvp_calls"]
        it.check(total == run_jvp, f"{key}: attempt JVPs + outside {total} != run jvp_calls {run_jvp}")
        jvp["/".join(key)] = {"run": run_jvp, "outside_attempts": rec["jvp_outside_attempts"],
                              "unattributed": run_jvp - total,
                              "per_solve_jvp_equals_matvecs": all(s["jvp_calls"] == s["solver"]["matvecs"]
                                                                  for s in rec["stages"])}
    it.numbers["jvp_closure"] = jvp
    return it


def item3(cells: dict, driver: dict, native: dict) -> Item:
    it = Item("3")
    exceeded_records = 0
    for label, t, _, _ in _solves(cells, driver):
        actual, requested = f(t["actual_residual"]), f(t["requested_target"])
        if t["reason"] != "Failed" and actual > requested:
            exceeded_records += 1
            it.check(t["attainment"] == "BudgetExceeded", f"{label}: accepted above target is {t['attainment']}")
    it.numbers["accepted_above_requested"] = exceeded_records
    for key, run in on_runs(driver):
        rec = run["records"]
        if rec is None:
            continue
        by_attempt: dict = {}
        for s in rec["stages"]:
            t = s["solver"]
            if t["reason"] != "Failed" and f(t["actual_residual"]) > f(t["requested_target"]):
                by_attempt[s["attempt"]] = by_attempt.get(s["attempt"], 0) + 1
        for a in rec["attempts"]:
            n = by_attempt.get(a["attempt"], 0)
            it.check(a["budget_exceeded"] == n,
                     f"{key} attempt {a['attempt']} ({a['outcome']}): budget_exceeded {a['budget_exceeded']} != {n}")
    reports = {s["id"]: s["observed"]["report"] for s in native["implementation_adversaries"]["stage_acceptance"]}
    for cid in S1_IDS:
        t = cells[("S1", cid)]["telemetry"]
        if cid in S1_EXCEEDED:
            it.check(t["attainment"] == "BudgetExceeded", f"S1/{cid}: {t['attainment']} (registered BudgetExceeded)")
            it.check(t["actual_residual"] == hx(reports[cid]["residual_norm"]),
                     f"S1/{cid}: actual {t['actual_residual']} != NATIVE residual_norm")
        else:
            it.check(t["attainment"] == "Met", f"S1/{cid}: {t['attainment']} (registered Met)")
    s4 = cells[("S4", "raised_target")]["telemetry"]
    it.check(s4["reason"] == "Converged" and s4["attainment"] == "BudgetExceeded",
             f"S4: {s4['reason']} / {s4['attainment']} (registered Converged / BudgetExceeded)")
    return it


def _native_matches(mine: Any, native: Any) -> bool:
    if isinstance(native, str) and native in ("NaN", "+Inf", "-Inf"):
        x = f(mine) if isinstance(mine, str) else None
        if x is None:
            return False
        return math.isnan(x) if native == "NaN" else x == (math.inf if native == "+Inf" else -math.inf)
    if type(native) is float:
        return mine == hx(native)
    if type(native) is int and isinstance(mine, str) and HEX64.match(mine):
        return mine == hx(float(native))
    return mine == native


def item4(cells: dict, driver: dict, native: dict) -> Item:
    it = Item("4")
    for label, t, _, _ in _solves(cells, driver):
        req, raised, tight = f(t["requested_target"]), f(t["raised_target"]), f(t["tightened_target"])
        actual, floor = f(t["actual_residual"]), f(t["attainable_floor"])
        it.check(req <= raised, f"{label}: requested {req!r} > raised {raised!r}")
        it.check(tight <= raised, f"{label}: tightened {tight!r} > raised {raised!r}")
        it.check(t["verified_bound"] == "Unavailable", f"{label}: verified_bound {t['verified_bound']}")
        it.check(t["attainment"] == classify(actual, req), f"{label}: attainment {t['attainment']} inconsistent")
        if t["reason"] == "Converged":
            it.check(actual <= tight, f"{label}: Converged with actual {actual!r} > tightened {tight!r}")
        elif t["reason"] in ("StallAccepted", "FloorAccepted"):
            it.check(tight < actual <= floor, f"{label}: {t['reason']} without tightened < actual <= floor")
        elif t["reason"] == "FallbackAccepted":
            it.check(t["guard_abort"] is not None or t["budget_exhausted"] is True,
                     f"{label}: FallbackAccepted without a guard abort or budget exhaustion")
    # Solver cells: telemetry = report.
    for (cell, cid), x in sorted(cells.items()):
        t, r = x["telemetry"], x["report"]
        label = f"{cell}/{cid}"
        for a, b in (("reason", "outcome"), ("raised_target", "threshold"), ("tightened_target", "final_threshold"),
                     ("actual_residual", "residual_norm"), ("columns", "columns"), ("matvecs", "matvecs"),
                     ("residual_jvps", "true_residuals"), ("nu_flops", "nu_flops"), ("guard_abort", "guard_abort"),
                     ("budget_exhausted", "budget_exhausted")):
            it.check(t[a] == r[b], f"{label}: telemetry {a} {t[a]!r} != report {b} {r[b]!r}")
        it.check(t["heuristic_gain"]["nu_max"] == r["nu_max"], f"{label}: nu_max differs")
    # S1 reports = NATIVE.
    reports = {s["id"]: s["observed"]["report"] for s in native["implementation_adversaries"]["stage_acceptance"]}
    for cid in S1_IDS:
        mine = cells[("S1", cid)]["report"]
        for k in NATIVE_REPORT_KEYS:
            it.check(_native_matches(mine[k], reports[cid][k]),
                     f"S1/{cid}: report {k} {mine[k]!r} != NATIVE {reports[cid][k]!r}")
    # Registered solver-cell outcomes.
    def outcome(cid_cell, outcome_, failure=None, attainment=None, evaluations=None):
        x = cells[cid_cell]
        r, t = x["report"], x["telemetry"]
        label = "/".join(cid_cell)
        it.check(r["outcome"] == outcome_ and r["failure"] == failure,
                 f"{label}: {r['outcome']}/{r['failure']} (registered {outcome_}/{failure})")
        if attainment is not None:
            it.check(t["attainment"] == attainment, f"{label}: {t['attainment']} (registered {attainment})")
        if evaluations is not None:
            it.check(t["fallback_evaluations"] == evaluations, f"{label}: {t['fallback_evaluations']} fallback calls")
    outcome(("S2", "overflowing_norms"), "Failed", "BudgetExhausted", evaluations=1)
    outcome(("S2", "finite_norm_control"), "Failed", "BudgetExhausted", evaluations=1)
    outcome(("S2", "loose_finite"), "FallbackAccepted", None, "BudgetExceeded", 1)
    outcome(("S2", "loose_homogeneous"), "FallbackAccepted", None, "BudgetExceeded", 1)
    for cid in ("contraction_unclassified", "contraction_classified"):
        outcome(("S3", cid), "FallbackAccepted", None, evaluations=1)
        it.check(cells[("S3", cid)]["report"]["guard_abort"] == "Contraction", f"S3/{cid}: not a contraction abort")
    outcome(("S5", "nonfinite"), "Failed", "NonFinite", evaluations=0)
    outcome(("S5", "breakdown"), "Failed", "Breakdown", evaluations=0)
    outcome(("S5", "budget_exhausted"), "Failed", "BudgetExhausted", evaluations=0)
    # Driver runs: records = StageSolveStatistics, one attempt record per attempt.
    for key, run in on_runs(driver):
        res, rec = run["result"], run["records"]
        if not it.check(res.get("ok") is True and rec is not None, f"{key}: run failed or has no records"):
            continue
        stats = res["stage_statistics"]
        stages, attempts = rec["stages"], rec["attempts"]
        it.check(len(stages) == stats["solves"], f"{key}: {len(stages)} records != solves {stats['solves']}")
        for reason, field in REASON_STAT.items():
            n = sum(1 for s in stages if s["solver"]["reason"] == reason)
            it.check(n == stats[field], f"{key}: {n} {reason} records != statistics {field} {stats[field]}")
        it.check(len(attempts) == res["attempts"], f"{key}: {len(attempts)} attempt records != {res['attempts']}")
        it.check([a["attempt"] for a in attempts] == list(range(len(attempts))), f"{key}: attempt indices")
        indices = [s["attempt"] for s in stages]
        it.check(indices == sorted(indices) and all(0 <= i < len(attempts) for i in indices),
                 f"{key}: stage records' attempt indices out of order or range")
        per: dict = {}
        for s in stages:
            per.setdefault(s["attempt"], []).append(s)
        for a in attempts:
            mine = per.get(a["attempt"], [])
            it.check(a["stages"] == len(mine), f"{key} attempt {a['attempt']}: stages {a['stages']} != {len(mine)}")
            for reason, field in REASON_ATTEMPT.items():
                n = sum(1 for s in mine if s["solver"]["reason"] == reason)
                it.check(a[field] == n, f"{key} attempt {a['attempt']}: {field} {a[field]} != {n}")
            ratio = math.nan
            for s in mine:
                t = s["solver"]
                if t["reason"] == "Failed":
                    continue
                r = div(f(t["actual_residual"]), f(t["requested_target"]))
                if math.isnan(ratio) or r > ratio:
                    ratio = r
            it.check(same(a["max_actual_over_requested"], ratio),
                     f"{key} attempt {a['attempt']}: max actual/requested {a['max_actual_over_requested']} != "
                     f"{hx(ratio)}")
            it.check([s["stage"] for s in mine] == list(range(len(mine))),
                     f"{key} attempt {a['attempt']}: stage indices {[s['stage'] for s in mine]}")
    return it


def item5(driver: dict) -> Item:
    it = Item("5")
    d1_failed_with_jvps = 0
    for key, run in on_runs(driver):
        res, rec = run["result"], run["records"]
        if rec is None or res.get("ok") is not True:
            it.check(False, f"{key}: no records")
            continue
        n = run["dimension"]
        coupled = run["options"]["policy"] in COUPLED_POLICIES
        per: dict = {}
        for s in rec["stages"]:
            per.setdefault(s["attempt"], []).append(s)
            t = s["solver"]
            if coupled:
                debit = f(s["tau_e"]) * (f(t["actual_residual"]) / math.sqrt(n))
                it.check(same(s["debit"], debit), f"{key}: stage debit {s['debit']} != recomputed {hx(debit)}")
            else:
                it.check(math.isnan(f(s["debit"])) and math.isnan(f(s["tau_e"])), f"{key}: non-coupled debit")
        committed = 0.0
        total = {k: 0 for k in WORK_KEYS}
        for a in rec["attempts"]:
            stages = per.get(a["attempt"], [])
            debit_all, debit_fallback = 0.0, 0.0
            for s in stages:
                t = s["solver"]
                if coupled and t["reason"] != "Failed" and t["attainment"] != "Met":
                    debit_all += f(s["debit"])
                if coupled and t["reason"] == "FallbackAccepted":
                    debit_fallback += f(s["debit"])
            it.check(a["debit_all"] == hx(debit_all), f"{key} attempt {a['attempt']}: debit_all != recomputed")
            it.check(a["debit_fallback_only"] == hx(debit_fallback),
                     f"{key} attempt {a['attempt']}: debit_fallback_only != recomputed")
            it.check(f(a["debit_all"]) >= f(a["debit_fallback_only"]),
                     f"{key} attempt {a['attempt']}: debit_all {f(a['debit_all'])!r} < debit_fallback_only "
                     f"{f(a['debit_fallback_only'])!r}")
            if a["outcome"] == "Committed":
                committed += f(a["debit_all"])
            w = a["work"]
            sums = {"krylov_columns": sum(s["solver"]["columns"] for s in stages),
                    "arnoldi_jvps": sum(s["solver"]["arnoldi_jvps"] for s in stages),
                    "residual_jvps": sum(s["solver"]["residual_jvps"] for s in stages),
                    "shadow_matvecs": sum(s["solver"]["uncounted_shadow_matvecs"] for s in stages),
                    "vector_copies": sum(s["solver"]["acceptance_vector_copies"] for s in stages)}
            for k, v in sums.items():
                it.check(w[k] == v, f"{key} attempt {a['attempt']}: work.{k} {w[k]} != stage sum {v}")
            for k in WORK_KEYS:
                total[k] += w[k]
            if key[0] == "D1" and a["outcome"] in ("RejectedError", "FailedLinear") and w["jvp_calls"] > 0:
                d1_failed_with_jvps += 1
        it.check(rec["committed_debit"] == hx(committed),
                 f"{key}: committed_debit {rec['committed_debit']} != recomputed {hx(committed)}")
        it.check(rec["work_all_attempts"] == total, f"{key}: work_all_attempts {rec['work_all_attempts']} != {total}")
        # The charge log.
        charged = [a for a in rec["attempts"] if a["fallback_accepted"] > 0 and not math.isnan(f(a["error_norm"]))]
        charges = res["charges"]
        if it.check(len(charged) == len(charges), f"{key}: {len(charged)} charged attempts != {len(charges)} charges"):
            for a, c in zip(charged, charges):
                it.check([a["t"], a["h"], a["error_norm"]] == c[:3],
                         f"{key} attempt {a['attempt']}: charge row t/h/error differ")
                it.check(a["debit_fallback_only"] == c[3],
                         f"{key} attempt {a['attempt']}: debit_fallback_only {a['debit_fallback_only']} != charge "
                         f"{c[3]}")
    it.numbers["d1_failed_attempts_with_jvps"] = d1_failed_with_jvps
    it.check(d1_failed_with_jvps >= 1, "D1 has no RejectedError/FailedLinear attempt with nonzero JVPs")
    return it


def item6(driver: dict) -> Item:
    it = Item("6")
    run = driver[("D2", "as04-d2-linear-2", "CoupledGuarded2", True)]
    rec = run["records"]
    if it.check(rec is not None and run["result"].get("ok") is True, "D2: no records"):
        attempts = rec["attempts"]
        generation = 0
        epochs = []
        for a in attempts:
            generation += 1 if a["relinearized"] else 0
            it.check(a["operator_epoch"]["generation"] == generation,
                     f"D2 attempt {a['attempt']}: generation {a['operator_epoch']['generation']} != {generation}")
            if a["outcome"] in ("Committed", "RejectedError"):
                expected = STAGES - 1 + (1 if a["relinearized"] else 0)
                it.check(a["work"]["rhs_calls"] == expected,
                         f"D2 attempt {a['attempt']}: rhs_calls {a['work']['rhs_calls']} != {expected} "
                         "(re-linearization not independently confirmed)")
            epochs.append(a["operator_epoch"]["model_epoch"])
        for s in rec["stages"]:
            it.check(s["operator_epoch"] == attempts[s["attempt"]]["operator_epoch"],
                     f"D2 stage record of attempt {s['attempt']}: epoch differs from its attempt's")
        it.numbers["d2_attempts"] = len(attempts)
        it.numbers["d2_model_epochs"] = epochs
        it.numbers["d2_relinearized"] = [a["relinearized"] for a in attempts]
        it.numbers["d2_outcomes"] = [a["outcome"] for a in attempts]
        changes = [k for k in range(1, len(epochs)) if epochs[k] != epochs[k - 1]]
        it.check(len(epochs) >= 4 and changes == [3] and epochs[0] is not None,
                 f"D2: model_epoch changes at attempts {changes} (registered: exactly the fourth, index 3)")
        if len(attempts) >= 4:
            it.check(attempts[3]["relinearized"], "D2: the fourth attempt did not re-linearize")
    for key, run in on_runs(driver):
        rec = run["records"]
        if rec is None:
            continue
        kind = "ScaledWrms" if run["options"]["policy"] in COUPLED_POLICIES else "UnscaledL2"
        bad = sum(1 for s in rec["stages"] if s["metric"]["kind"] != kind)
        it.check(bad == 0, f"{key}: {bad} records not {kind}")
        if run["options"]["policy"] == "ProjL2":
            it.numbers["projl2_unscaled_records"] = len(rec["stages"])
    return it


# ------------------------------------------------------------------------------------------- reported
def quantiles(values: list[float]) -> dict:
    if not values:
        return {"count": 0}
    v = sorted(values)
    q = lambda p: v[min(len(v) - 1, int(p * (len(v) - 1) + 0.5))]  # noqa: E731
    return {"count": len(v), "min": v[0], "median": q(0.5), "p90": q(0.9), "p99": q(0.99), "max": v[-1]}


def reported(cells: dict, driver: dict) -> dict:
    out: dict = {}
    for key, run in on_runs(driver):
        rec = run["records"]
        if rec is None:
            continue
        reasons = {r: 0 for r in REASONS}
        attain = {a: 0 for a in ATTAINMENTS}
        by = {}
        ratios = []
        for s in rec["stages"]:
            t = s["solver"]
            reasons[t["reason"]] += 1
            attain[t["attainment"]] += 1
            by.setdefault(f"{t['reason']}/{t['attainment']}", 0)
            by[f"{t['reason']}/{t['attainment']}"] += 1
            if t["reason"] != "Failed":
                ratios.append(div(f(t["actual_residual"]), f(t["requested_target"])))
        crossing = []
        for a in rec["attempts"]:
            e = f(a["error_norm"])
            if not math.isnan(e) and e <= 1.0 and e + f(a["debit_all"]) > 1.0:
                crossing.append({"attempt": a["attempt"], "t": f(a["t"]), "error_norm": e,
                                 "debit_all": f(a["debit_all"])})
        stats = run["result"]["stage_statistics"]
        out["/".join(key)] = {
            "reasons": reasons, "attainments": attain, "reason_attainment": by,
            "non_converged_acceptances": reasons["StallAccepted"] + reasons["FloorAccepted"]
            + reasons["FallbackAccepted"],
            "converged_budget_exceeded": by.get("Converged/BudgetExceeded", 0),
            "actual_over_requested": quantiles(ratios),
            "committed_debit": f(rec["committed_debit"]), "g3_charge_sum": f(stats["charge_sum"]),
            "g3_charge_max": f(stats["charge_max"]), "charged_attempts": stats["charged_attempts"],
            "max_debit_all": max((f(a["debit_all"]) for a in rec["attempts"]), default=0.0),
            "attempts_norm_plus_debit_all_cross_1": len(crossing), "crossing_attempts": crossing[:20],
            "roundoff_floor_binds": stats["roundoff_floor_binds"],
            "memory_bytes": rec["memory_bytes"], "stage_records": len(rec["stages"]),
            "attempt_records": len(rec["attempts"]), "jvp_outside_attempts": rec["jvp_outside_attempts"],
            "work_all_attempts": rec["work_all_attempts"],
        }
    cells_out = {}
    for (cell, cid), x in sorted(cells.items()):
        t = x["telemetry"]
        cells_out[f"{cell}/{cid}"] = {"reason": t["reason"], "attainment": t["attainment"],
                                      "actual": f(t["actual_residual"]), "requested": f(t["requested_target"]),
                                      "copies": t["acceptance_vector_copies"],
                                      "evaluations": x["evaluations"]}
    out["solver_cells"] = cells_out
    return out


def predictions(rep: dict, items: dict) -> dict:
    rob = rep.get("R/robertson-4e10/1e-5/B3", {})
    rob9 = rep.get("R/robertson-4e10/1e-9/B3", {})
    return {
        "items_1_6_hold": all(v["pass"] for v in items.values()),
        "robertson_1e-5_B3_non_converged_acceptances": {"predicted": 302,
                                                        "observed": rob.get("non_converged_acceptances")},
        "robertson_1e-5_B3_converged_budget_exceeded": {"predicted": "1..443",
                                                        "observed": rob.get("converged_budget_exceeded")},
        "robertson_debit_all_exceeds_g3_charge": {
            "predicted": True,
            "observed_1e-5": rob.get("committed_debit", 0.0) > rob.get("g3_charge_sum", 0.0) if rob else None,
            "observed_1e-9": rob9.get("committed_debit", 0.0) > rob9.get("g3_charge_sum", 0.0) if rob9 else None},
        "robertson_1e-5_no_crossing": {"predicted": 0,
                                       "observed": rob.get("attempts_norm_plus_debit_all_cross_1")},
        "robertson_1e-9_crossing": {"predicted": "uncertain",
                                    "observed": rob9.get("attempts_norm_plus_debit_all_cross_1")},
    }


# ------------------------------------------------------------------------------------------- driver
def evaluate(runs: dict, native: dict, alg06: dict, audit: dict | None, suites: dict | None) -> dict:
    """Validated documents -> the gate (items, verdict, reported numbers)."""
    cells, driver = validate(runs)
    validate_references(native, alg06)
    items = {it.name: it.value() for it in (item1a(driver, audit, suites), item1b(driver, alg06),
                                            item2(cells, driver), item3(cells, driver, native),
                                            item4(cells, driver, native), item5(driver), item6(driver))}
    verdict = "PASS" if all(v["pass"] for v in items.values()) else "FAIL"
    rep = reported(cells, driver)
    return {
        "verdict": verdict,
        "items": items,
        "kill_rule": ("telemetry not adopted (item 1a failed); no telemetry number may be used downstream"
                      if not items["1a"]["pass"] else "not triggered"),
        "anchor": "anchor drift (item 1b failed; kill rule not applied)" if not items["1b"]["pass"] else "reproduced",
        "hold_rule": ("items 3-4 failed: the reviewer decides whether completeness needs a decision change"
                      if not (items["3"]["pass"] and items["4"]["pass"]) else "not triggered"),
        "reported": rep,
        "predictions": predictions(rep, items),
    }


def load_runs(path: Path) -> tuple[Any, str]:
    try:
        data = path.read_bytes()
    except OSError as exc:
        raise evidence.ValidationError([f"runs: cannot read {path}: {exc}"]) from None
    digest = hashlib.sha256(data).hexdigest()
    if path.suffix == ".gz":
        try:
            data = gzip.decompress(data)
        except OSError as exc:
            raise evidence.ValidationError([f"runs: gzip: {exc}"]) from None
    return evidence.loads_strict(data), digest


def main(argv: list[str] | None = None, *, suites_runner: Callable[[], dict] = run_test_suites) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--runs", type=Path, required=True)
    ap.add_argument("--native", type=Path, required=True)
    ap.add_argument("--alg06", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args(argv)
    if args.output.exists():
        print(f"immutable output exists: {args.output}", file=sys.stderr)
        return 2
    header = {"schema": SCHEMA, "node": NODE, "evidence_schema": evidence.VERSION,
              "inputs": {"runs": str(args.runs), "native": str(args.native), "alg06": str(args.alg06)}}
    try:
        docs, digests, _ = evidence._load_all([("native", args.native, NATIVE_SHA256),
                                               ("alg06_runs", args.alg06, ALG06_RUNS_SHA256)])
        runs, runs_digest = load_runs(args.runs)
        digests = {**digests, "runs": runs_digest}
        validate(runs)
        validate_references(docs["native"], docs["alg06_runs"])
        provenance = check_provenance(runs, Path(__file__).resolve())
    except evidence.ValidationError as exc:
        result = {**header, "verdict": evidence.INVALID, "reasons": list(exc.reasons)[:evidence.MAX_REASONS]}
        args.output.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
        print(f"INVALID: {exc}", file=sys.stderr)
        return evidence.INVALID_EXIT_CODE
    audit = audit_test_files(provenance["source_commit"])
    suites = suites_runner()
    result = {**header, "sha256": digests, "provenance": provenance,
              **evaluate(runs, docs["native"], docs["alg06_runs"], audit, suites)}
    args.output.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    print(f"{result['verdict']}: " + ", ".join(f"{k} {'PASS' if v['pass'] else 'FAIL'}"
                                                for k, v in result["items"].items()))
    return 0 if result["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
