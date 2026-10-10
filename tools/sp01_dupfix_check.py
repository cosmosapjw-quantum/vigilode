#!/usr/bin/env python3
"""Gate of research node research/sp01_dupfix_adoption_20261010 (SP01; see its PREREGISTRATION.md).

Inputs: RUNS.json (`export_runs` of crates/rodas5p-integrators/tests/sp01_dupfix_adoption.rs) and PROFILE.json
(tools/sp01_dupfix_profile.py). The checker also runs, on the checked-out tree, the registered kernel contract
command (`cargo test --locked -p rodas5p-krylov --test dupfix_promotion_contract`) and the pre-existing test suites
of item 4, and reads git for the test-file audit. Accountings: `recompute_final` (v1, legacy) and `reuse_confirmed`
(v2). Cells: 44 (uform_into 14, uform_default 14, kform_integrate 8, kform_step 8).

Evidence validation first (re-audit AS03): the inputs are parsed and typed with tools/evidence_schema_v2.py (strict
JSON, exact key sets, the exact registered cell and arm sets with unique raw keys, canonical binary64 hex vectors of
the cell dimension, correctly typed nonnegative counts, the SP01 receipt of the decision-rule trial, and ALG04's
RUNS.json bound to its committed SHA-256 for the reported reproduction). Rejected evidence is written with verdict
INVALID and its reasons and the checker exits with status 2; INVALID is neither PASS nor FAIL.

Gate (PASS iff all four items pass; everything else FAIL, with every number preserved):

1. Parity. In every cell the two accountings' records are equal in every field except `counters`: output times and
   final state bits, ok/success/message, attempts, accepted and rejected steps, the per-attempt error norms (K-form)
   and failed attempts (kform_step), and the stage-solve log (count, total iterations and SHA-256 of every
   successful stage solve's reported residual norm, relative residual and iterations, in order: the report residual
   norms). Their counters `linear_solve_failures` and `linear_iterations` are equal (item 2 compares every counter
   but the accounting ones). And the kernel boundary contract test passes (zero RHS, nonzero x0 within threshold,
   budget exhaustion, happy breakdown confirmed and not, Jacobi, WRMS scale, restart below the iteration count, both
   kernels: same solution, outcome and residual norm).
2. Exact accounting. In every cell `jvp_vectors` and `jvp_calls` of v1 minus v2 both equal the observer's
   `confirmed_exits` of the v1 run (the independently computed count of stage solves that left GMRES on a confirmed
   true residual of the same iterate), every counter other than the JVP/matvec totals and the diagnostic category
   (`jvp_calls`, `jvp_vectors`, `linear_matvec_vectors`, `mass_matvecs`, `jacobian_matvecs`,
   `diagnostic_matvecs`) is equal, and the observed (wrapped-problem) runs reproduce the plain runs' records bit for
   bit under both accountings (otherwise the count does not describe the plain run).
3. No instruction regression. On every profiled cell (uform_into van der Pol, HIRES, Brusselator-50 at 1e-6;
   kform_integrate and kform_step HIRES at 1e-6) Ir per trajectory (callgrind, 2 minus 1 integrations, same
   binary) under v2 is <= 1.000 x v1; every callgrind run is deterministic (repeated 1-integration total equal) and
   printed the native record; and each profiled run's native work (attempts, counters, final state, logged solves)
   equals the RUNS record of its cell and accounting.
4. Contract. (a) No pre-existing test file is modified: relative to the branch base 0283c1f (the tree the node
   started from), every changed path under crates/*/tests/ is an addition, and the `#[cfg(test)]` tail of every
   changed crates/*/src file is unchanged. (b) The default follows the registered decision rule: the committed
   trial receipt (DEFAULT_TRIAL.json, every pre-existing non-ignored test run with `ReuseConfirmed` as the default of
   both kernels) lists failing pre-existing tests iff the kernels' `ResidualAccounting::DEFAULT` is
   `RecomputeFinal`, and RUNS records that default. (c) Every pre-existing test passes under that default: the
   workspace test suite (all targets), the doc tests, the three rodas5p-integrators feature suites and the Python
   tool tests all exit 0 on the checked-out tree.

Interpretations fixed before the recorded run (this checker is committed before RUNS.json and PROFILE.json exist):
- ReuseConfirmed reuses only a loop exit on an operator-computed true residual of the current (nonzero) iterate
  after at least one restart cycle. The zero-RHS / zero-iterate exit and the nonzero-x0 exit before any cycle keep
  the recomputation, as the registration lists them among the exits that keep it.
- The independent count: an observer of the problem's JVP callback on a wrapped copy of the problem; a maximal run
  of >= 2 consecutive applications to the same nonzero input at the same linearization (no RHS evaluation in
  between) is one recomputed final residual; it is a confirmed exit when the application before the run acted on a
  unit 2-norm vector (an Arnoldi basis vector) at the same linearization.
- "JVP" is `jvp_vectors`; `jvp_calls` must give the same difference.
- The registered K-form "default library path (integrate / sequential_matrix_free_step with GMRES)" is run on both
  entry points (`kform_integrate`: integrate_sequential_matrix_free_adaptive_observed, the protected WRMS-forcing
  driver; `kform_step`: the adaptive loop over sequential_matrix_free_step), LinearSolverConfig::default() with
  GMRES. The K-form profiled cell is profiled on both entry points; both are gated.

Reported, never gated: per-cell JVP ratios v2/v1 (per trajectory = per attempt, attempts being equal) against the
predictions; the diagnostic and matvec-vector differences; the observer decomposition of v1's final residuals
(confirmed + nonzero-x0-before-any-cycle + zero iterate) and the v2 observer count (expected 0); ALG04's
reproduction (uform_into v1 against ALG04's `legacy` C1 records and `dup_fix` against ALG04's `dup_fix`); Ir
ratios against the prediction (0.80-1.00). Counted work and same-binary instructions only; no wall-time claim. The
output file is immutable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

SCHEMA = "vigilode-sp01-dupfix-check-v1"
NODE = "sp01_dupfix_adoption_20261010"
NODE_DIR = ROOT / "research" / NODE
BASE_COMMIT = "0283c1f7f2752d2b6b6b3b56847e73284a6dd67f"
ALG04_RUNS = ROOT / "research" / "alg04_coupled_target_v2_20261010" / "RUNS.json"
ALG04_RUNS_SHA256 = "0512f858f6c1cba4a5940452e383e1f0f565bee1e9c25e83264d10b8cf14ff44"
TRIAL = NODE_DIR / "DEFAULT_TRIAL.json"

V1, V2 = "recompute_final", "reuse_confirmed"
ACCOUNTINGS = (V1, V2)
ACCOUNTING_IDS = {V1: "gmres-residual-accounting-recompute-final-v1",
                  V2: "gmres-residual-accounting-reuse-confirmed-v2"}
RTOLS = (1e-06, 1e-08)
SPD07 = ("robertson", "van-der-pol-mu1000", "hires", "brusselator-1d-50", "prothero-robinson-forced", "quadratic-4",
         "brusselator-1d-160")
KFORM = ("robertson", "van-der-pol-mu1000", "hires", "brusselator-1d-50")
PATHS = ("uform_into", "uform_default", "kform_integrate", "kform_step")
REGISTERED_CELLS = ([(p, c, r) for p in ("uform_into", "uform_default") for c in SPD07 for r in RTOLS]
                    + [(p, c, r) for p in ("kform_integrate", "kform_step") for c in KFORM for r in RTOLS])
PROFILED = (("uform_into", "van-der-pol-mu1000", 1e-06), ("uform_into", "hires", 1e-06),
            ("uform_into", "brusselator-1d-50", 1e-06), ("kform_integrate", "hires", 1e-06),
            ("kform_step", "hires", 1e-06))
ACCOUNTING_COUNTERS = frozenset(("jvp_calls", "jvp_vectors", "linear_matvec_vectors", "mass_matvecs",
                                 "jacobian_matvecs", "diagnostic_matvecs"))
IR_LIMIT = 1.000

SOLVE_LOG_KEYS = frozenset(("count", "iterations", "sha256"))
OBSERVER_KEYS = frozenset(("jvp_applications", "confirmed_exits", "unconfirmed_duplicates", "zero_inputs",
                           "other_events"))
RECORD_KEYS = {
    "uform_into": evidence.ADAPTIVE_PLAIN_KEYS | {"solve_log"},
    "uform_default": evidence.ADAPTIVE_PLAIN_KEYS | {"solve_log"},
    "kform_integrate": frozenset(("ok", "success", "message", "t", "y_last", "attempts", "accepted", "rejected",
                                  "linear_solve_failures", "error_norms", "internal_steps", "output_clipped_steps",
                                  "counters", "solve_log")),
    "kform_step": frozenset(("ok", "success", "t_last", "y_last", "attempts", "accepted", "rejected", "error_norms",
                             "failed_attempts", "counters", "solve_log")),
}
INT_FIELDS = ("attempts", "accepted", "rejected", "state_reuses", "internal_steps", "output_clipped_steps",
              "linear_solve_failures")
HEX_ANY = re.compile(r"^[0-9a-f]{16}$")
SHA = re.compile(r"^[0-9a-f]{64}$")
PROFILE_KEYS = frozenset(("schema", "node", "commit", "tree_clean", "valgrind", "binary", "binary_sha256", "build",
                          "environment", "protocol", "cells", "entries"))
PROFILE_ENTRY_KEYS = frozenset(("path", "problem", "rtol", "accounting", "native", "ir_run1", "ir_run2",
                                "ir_run1_repeat", "callgrind_deterministic", "callgrind_records_match_native",
                                "ir_per_trajectory", "ir_per_attempt"))
NATIVE_KEYS = frozenset(("success", "attempts", "accepted_steps", "rejected_steps", "counters", "final_state",
                         "logged_solves", "path", "problem", "rtol", "accounting", "repetitions"))
TRIAL_KEYS = frozenset(("node", "trial", "tree", "commands", "steps", "failing_pre_existing_tests", "outcome"))

# Predictions (registered; reported only).
PREDICTED_UFORM_INTO = {"van-der-pol-mu1000": (0.75, 0.75), "robertson": (0.77, 0.77), "hires": (0.89, 0.89),
                        "prothero-robinson-forced": (0.63, 0.65), "brusselator-1d-50": (0.98, 0.98),
                        "brusselator-1d-160": (0.98, 0.98)}
PREDICTED_KFORM = (0.80, 0.95)
PREDICTED_IR = (0.80, 1.00)

# Item 4(c): the pre-existing suites (the same commands as the decision-rule trial) and the Python tool tests.
SUITES = [
    ("test-workspace", ["cargo", "test", "--workspace", "--all-targets", "--locked", "--no-fail-fast"]),
    ("test-doc", ["cargo", "test", "--workspace", "--doc", "--locked", "--no-fail-fast"]),
    ("test-research", ["cargo", "test", "-p", "rodas5p-integrators", "--features", "audit2-research", "--locked",
                       "--no-fail-fast"]),
    ("test-bateman", ["cargo", "test", "-p", "rodas5p-integrators", "--features", "audit2-bateman-authority",
                      "--locked", "--no-fail-fast"]),
    ("test-stagecert", ["cargo", "test", "-p", "rodas5p-integrators", "--features", "audit2-stage-certificate",
                        "--locked", "--no-fail-fast"]),
    ("python-tools", [sys.executable, "-m", "unittest", "discover", "-s", "tools", "-p", "test_*.py"]),
]
CONTRACT = ["cargo", "test", "--locked", "-p", "rodas5p-krylov", "--test", "dupfix_promotion_contract"]


# ------------------------------------------------------------------------------------------- validation
def _check_record(c, path: str, rec, kind: str, dimension: int) -> None:
    if not c.dict_(path, rec) or not c.bool_(f"{path}.ok", rec.get("ok")):
        return
    if not rec["ok"]:
        if c.keys(path, rec, {"ok", "error"}):
            c.str_(f"{path}.error", rec["error"])
        return
    if not c.keys(path, rec, RECORD_KEYS[kind]):
        return
    c.bool_(f"{path}.success", rec["success"])
    if "message" in rec:
        c.str_(f"{path}.message", rec["message"], nonempty=False)
    for k in INT_FIELDS:
        if k in rec:
            c.int_(f"{path}.{k}", rec[k])
    if "t" in rec:
        c.hex_vector(f"{path}.t", rec["t"])
    if "t_last" in rec:
        c.hex_vector(f"{path}.t_last", rec["t_last"], 1)
    c.hex_vector(f"{path}.y_last", rec["y_last"], dimension)
    if "error_norms" in rec and c.list_(f"{path}.error_norms", rec["error_norms"]):
        bad = [i for i, v in enumerate(rec["error_norms"]) if not (isinstance(v, str) and HEX_ANY.match(v))]
        if bad:
            c.fail(f"{path}.error_norms[{bad[0]}]", "not a 16-digit lowercase binary64 hex string")
    if "failed_attempts" in rec and c.list_(f"{path}.failed_attempts", rec["failed_attempts"]):
        for i, v in enumerate(rec["failed_attempts"]):
            c.str_(f"{path}.failed_attempts[{i}]", v)
    c.counters(f"{path}.counters", rec["counters"], evidence.COUNTERS_FULL)
    log = rec["solve_log"]
    if c.keys(f"{path}.solve_log", log, SOLVE_LOG_KEYS):
        c.int_(f"{path}.solve_log.count", log["count"])
        c.int_(f"{path}.solve_log.iterations", log["iterations"])
        if not (isinstance(log["sha256"], str) and SHA.match(log["sha256"])):
            c.fail(f"{path}.solve_log.sha256", "not a 64-digit lowercase SHA-256")


def validate(runs_path: Path, profile_path: Path, trial_path: Path = TRIAL, alg04_path: Path = ALG04_RUNS,
             alg04_sha256: str | None = ALG04_RUNS_SHA256):
    """Strict parsing and typing of RUNS, PROFILE, the trial receipt and ALG04's RUNS (bound to its digest)."""
    docs, digests, paths = evidence._load_all([
        ("runs", runs_path, None),
        ("profile", profile_path, None),
        ("trial", trial_path, None),
        ("alg04_runs", alg04_path, alg04_sha256),
    ])
    c = evidence._Checks()
    runs, profile, trial = docs["runs"], docs["profile"], docs["trial"]
    # RUNS
    if c.keys("runs", runs, {"node", "export", "accountings", "default_accounting", "uform_budget",
                             "kform_linear_config", "rows"}):
        c.equal("runs.node", runs["node"], NODE)
        c.equal("runs.export", runs["export"], "export_runs")
        c.equal("runs.accountings", runs["accountings"], ACCOUNTING_IDS)
        if runs["default_accounting"] not in ACCOUNTING_IDS.values():
            c.fail("runs.default_accounting", f"unknown accounting {runs['default_accounting']!r}")
        c.int_("runs.uform_budget", runs["uform_budget"], minimum=1)
        c.str_("runs.kform_linear_config", runs["kform_linear_config"])
        index = evidence._rows(c, "runs.rows", runs["rows"], {"path": "str", "case": "str", "rtol": "float+",
                                                              "dimension": "int+"},
                               lambda r: (r["path"], r["case"], r["rtol"]))
        if index and sorted(index) != sorted(REGISTERED_CELLS):
            missing = sorted(set(REGISTERED_CELLS) - set(index))
            extra = sorted(set(index) - set(REGISTERED_CELLS))
            c.fail("runs.rows", f"cell set differs from the registration (missing {missing}, unexpected {extra})")
        for key, row in index.items():
            p = f"runs.rows{list(key)}"
            if key[0] not in PATHS or not c.keys(p, row, {"path", "case", "rtol", "dimension", "arms",
                                                          "observed_arms", "observer"}):
                continue
            arms = set(ACCOUNTINGS) | ({"dup_fix"} if key[0] == "uform_into" else set())
            if c.keys(f"{p}.arms", row["arms"], arms):
                for arm in sorted(arms):
                    _check_record(c, f"{p}.arms.{arm}", row["arms"][arm], key[0], row["dimension"])
            if c.keys(f"{p}.observed_arms", row["observed_arms"], ACCOUNTINGS):
                for arm in ACCOUNTINGS:
                    _check_record(c, f"{p}.observed_arms.{arm}", row["observed_arms"][arm], key[0],
                                  row["dimension"])
            if c.keys(f"{p}.observer", row["observer"], ACCOUNTINGS):
                for arm in ACCOUNTINGS:
                    o = row["observer"][arm]
                    if c.keys(f"{p}.observer.{arm}", o, OBSERVER_KEYS):
                        for k in sorted(OBSERVER_KEYS):
                            c.int_(f"{p}.observer.{arm}.{k}", o[k])
    # PROFILE
    if c.keys("profile", profile, PROFILE_KEYS):
        c.equal("profile.node", profile["node"], NODE)
        c.equal("profile.schema", profile["schema"], "vigilode-sp01-dupfix-profile-v1")
        c.bool_("profile.tree_clean", profile["tree_clean"])
        for k in ("commit", "valgrind", "binary", "binary_sha256", "build", "protocol"):
            c.str_(f"profile.{k}", profile[k])
        if c.list_("profile.entries", profile["entries"], nonempty=True):
            keys = []
            for i, e in enumerate(profile["entries"]):
                p = f"profile.entries[{i}]"
                if not c.keys(p, e, PROFILE_ENTRY_KEYS):
                    continue
                if not (c.str_(f"{p}.path", e["path"]) and c.str_(f"{p}.problem", e["problem"])
                        and c.float_(f"{p}.rtol", e["rtol"], positive=True)
                        and c.str_(f"{p}.accounting", e["accounting"])):
                    continue
                keys.append((e["path"], e["problem"], e["rtol"], e["accounting"]))
                for k in ("ir_run1", "ir_run2", "ir_run1_repeat"):
                    c.int_(f"{p}.{k}", e[k], minimum=1)
                c.int_(f"{p}.ir_per_trajectory", e["ir_per_trajectory"], minimum=None)
                c.float_(f"{p}.ir_per_attempt", e["ir_per_attempt"])
                c.bool_(f"{p}.callgrind_deterministic", e["callgrind_deterministic"])
                c.bool_(f"{p}.callgrind_records_match_native", e["callgrind_records_match_native"])
                n = e["native"]
                if c.keys(f"{p}.native", n, NATIVE_KEYS):
                    c.bool_(f"{p}.native.success", n["success"])
                    for k in ("attempts", "accepted_steps", "rejected_steps", "logged_solves", "repetitions"):
                        c.int_(f"{p}.native.{k}", n[k])
                    c.counters(f"{p}.native.counters", n["counters"], evidence.COUNTERS_FULL)
                    c.hex_vector(f"{p}.native.final_state", n["final_state"])
            expected = sorted((a, b, r, acc) for a, b, r in PROFILED for acc in ACCOUNTINGS)
            if len(keys) != len(set(keys)) or sorted(keys) != expected:
                c.fail("profile.entries", f"profiled set {sorted(keys)} != registered {expected}")
    # Trial receipt
    if c.keys("trial", trial, TRIAL_KEYS):
        c.equal("trial.node", trial["node"], NODE)
        c.str_("trial.trial", trial["trial"])
        c.str_("trial.tree", trial["tree"])
        if c.list_("trial.failing_pre_existing_tests", trial["failing_pre_existing_tests"]):
            for i, t in enumerate(trial["failing_pre_existing_tests"]):
                c.str_(f"trial.failing_pre_existing_tests[{i}]", t)
        if trial["outcome"] not in ("default", "versioned"):
            c.fail("trial.outcome", f"unknown outcome {trial['outcome']!r}")
        if c.list_("trial.steps", trial["steps"], nonempty=True):
            for i, s in enumerate(trial["steps"]):
                p = f"trial.steps[{i}]"
                if c.keys(p, s, {"step", "command", "exit", "passed", "failed", "ignored", "failing_tests"}):
                    for k in ("exit", "passed", "failed", "ignored"):
                        c.int_(f"{p}.{k}", s[k], minimum=None if k == "exit" else 0)
    c.done()
    return evidence.Evidence("SP01", docs, digests, paths)


# ----------------------------------------------------------------------------------------------- gates
def rows_by_key(runs):
    return {(r["path"], r["case"], r["rtol"]): r for r in runs["rows"]}


def item_parity(rows, contract):
    cells, failures = [], []
    for key in REGISTERED_CELLS:
        a1, a2 = rows[key]["arms"][V1], rows[key]["arms"][V2]
        differing = sorted(k for k in set(a1) | set(a2) if k != "counters" and a1.get(k) != a2.get(k))
        if a1.get("ok") and a2.get("ok"):
            for k in ("linear_solve_failures", "linear_iterations"):
                if a1["counters"][k] != a2["counters"][k]:
                    differing.append(f"counters.{k}")
        ok = not differing and bool(a1.get("ok")) == bool(a2.get("ok"))
        cells.append({"cell": list(key), "pass": ok, "differing": differing,
                      "attempts": a1.get("attempts"), "solve_log": a1.get("solve_log")})
        if not ok:
            failures.append(list(key))
    return {"pass": not failures and contract["pass"], "cells_checked": len(cells), "failing_cells": failures,
            "kernel_boundary_contract": contract, "cells": cells}


def counter_delta(c1, c2):
    return {k: c1[k] - c2[k] for k in sorted(c1) if c1[k] != c2[k]}


def item_accounting(rows):
    cells, failures = [], []
    for key in REGISTERED_CELLS:
        row = rows[key]
        a1, a2 = row["arms"][V1], row["arms"][V2]
        count = row["observer"][V1]["confirmed_exits"]
        reasons = []
        bound = {acc: row["observed_arms"][acc] == row["arms"][acc] for acc in ACCOUNTINGS}
        if not all(bound.values()):
            reasons.append(f"observed runs differ from the plain runs: {bound}")
        if not (a1.get("ok") and a2.get("ok")):
            reasons.append("a run failed")
            cells.append({"cell": list(key), "pass": False, "reasons": reasons})
            failures.append(list(key))
            continue
        c1, c2 = a1["counters"], a2["counters"]
        delta = counter_delta(c1, c2)
        for k in ("jvp_vectors", "jvp_calls"):
            if c1[k] - c2[k] != count:
                reasons.append(f"{k} difference {c1[k] - c2[k]} != confirmed exits {count}")
        others = sorted(k for k in delta if k not in ACCOUNTING_COUNTERS)
        if others:
            reasons.append(f"other counters differ: {others}")
        o1 = row["observer"][V1]
        decomposition = o1["confirmed_exits"] + o1["unconfirmed_duplicates"] + o1["zero_inputs"]
        cells.append({
            "cell": list(key), "pass": not reasons, "reasons": reasons,
            "confirmed_exits": count, "delta": delta,
            "jvp_v1": c1["jvp_vectors"], "jvp_v2": c2["jvp_vectors"],
            "jvp_ratio_v2_over_v1": c2["jvp_vectors"] / c1["jvp_vectors"] if c1["jvp_vectors"] else None,
            "diagnostic_v1": c1["diagnostic_matvecs"], "diagnostic_v2": c2["diagnostic_matvecs"],
            "observer_v1": o1, "observer_v2": row["observer"][V2],
            "v1_final_residuals_decomposed": decomposition == c1["diagnostic_matvecs"],
        })
        if reasons:
            failures.append(list(key))
    return {"pass": not failures, "cells_checked": len(cells), "failing_cells": failures, "cells": cells}


def item_instructions(profile, rows):
    entries = {(e["path"], e["problem"], e["rtol"], e["accounting"]): e for e in profile["entries"]}
    cells, failures = [], []
    for path, problem, rtol in PROFILED:
        e1, e2 = entries[(path, problem, rtol, V1)], entries[(path, problem, rtol, V2)]
        reasons = []
        for acc, e in ((V1, e1), (V2, e2)):
            if not e["callgrind_deterministic"]:
                reasons.append(f"{acc}: callgrind not deterministic")
            if not e["callgrind_records_match_native"]:
                reasons.append(f"{acc}: a callgrind run printed a different record")
            rec = rows[(path, problem, rtol)]["arms"][acc]
            n = e["native"]
            binding = {
                "attempts": n["attempts"] == rec.get("attempts"),
                "accepted": n["accepted_steps"] == rec.get("accepted"),
                "rejected": n["rejected_steps"] == rec.get("rejected"),
                "counters": n["counters"] == rec.get("counters"),
                "final_state": n["final_state"] == rec.get("y_last"),
                "logged_solves": n["logged_solves"] == (rec.get("solve_log") or {}).get("count"),
                "success": n["success"] == rec.get("success"),
            }
            if not all(binding.values()):
                reasons.append(f"{acc}: profiled run differs from RUNS: {binding}")
        ratio = e2["ir_per_trajectory"] / e1["ir_per_trajectory"] if e1["ir_per_trajectory"] > 0 else None
        if ratio is None or not ratio <= IR_LIMIT:
            reasons.append(f"Ir ratio {ratio} not <= {IR_LIMIT}")
        cells.append({"cell": [path, problem, rtol], "pass": not reasons, "reasons": reasons,
                      "ir_per_trajectory_v1": e1["ir_per_trajectory"], "ir_per_trajectory_v2": e2["ir_per_trajectory"],
                      "ir_ratio_v2_over_v1": ratio, "ir_saved_per_trajectory": e1["ir_per_trajectory"]
                      - e2["ir_per_trajectory"], "attempts": e1["native"]["attempts"],
                      "within_predicted_0.80_1.00": ratio is not None and PREDICTED_IR[0] <= ratio <= PREDICTED_IR[1]})
        if reasons:
            failures.append([path, problem, rtol])
    return {"pass": not failures, "limit": IR_LIMIT, "failing_cells": failures, "cells": cells,
            "profile_commit": profile["commit"], "profile_tree_clean": profile["tree_clean"],
            "binary_sha256": profile["binary_sha256"], "valgrind": profile["valgrind"]}


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout


def test_file_audit():
    changes = [line.split("\t") for line in git("diff", "--name-status", f"{BASE_COMMIT}..HEAD").splitlines()]
    violations, added = [], []
    for change in changes:
        status, path = change[0], change[-1]
        old_path = change[1]
        if re.match(r"^crates/[^/]+/tests/", old_path) or re.match(r"^crates/[^/]+/tests/", path):
            if status == "A":
                added.append(path)
            else:
                violations.append(f"{status} {' '.join(change[1:])}")
        elif status == "M" and re.match(r"^crates/[^/]+/src/.*\.rs$", path):
            before = git("show", f"{BASE_COMMIT}:{path}")
            after = git("show", f"HEAD:{path}")
            marker = "#[cfg(test)]"
            if marker in before:
                if marker not in after or before[before.index(marker):] != after[after.index(marker):]:
                    violations.append(f"cfg(test) tail changed in {path}")
    return {"pass": not violations, "base": BASE_COMMIT, "added_test_files": added, "violations": violations}


def kernel_default() -> str:
    text = (ROOT / "crates/rodas5p-krylov/src/accounting.rs").read_text()
    m = re.search(r"pub const DEFAULT: Self = Self::(\w+);", text)
    return m.group(1) if m else "unknown"


def summarize(output: str) -> dict:
    passed = failed = ignored = 0
    for m in re.finditer(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", output):
        passed, failed, ignored = passed + int(m.group(1)), failed + int(m.group(2)), ignored + int(m.group(3))
    failing = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED$", output, flags=re.M)))
    py = re.search(r"^Ran (\d+) tests? in", output, flags=re.M)
    if py:
        failing = sorted(set(re.findall(r"^(?:FAIL|ERROR): (.+)$", output, flags=re.M)))
        failed = len(re.findall(r"^(?:FAIL|ERROR): ", output, flags=re.M))
        skipped = re.search(r"skipped=(\d+)", output)
        ignored = int(skipped.group(1)) if skipped else 0
        passed = int(py.group(1)) - failed - ignored
    return {"passed": passed, "failed": failed, "ignored": ignored, "failing_tests": failing}


def run(cmd):
    env = dict(os.environ, RAYON_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1")
    result = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True)
    output = result.stdout + result.stderr
    shown = ["python3", *cmd[1:]] if cmd[0] == sys.executable else cmd
    return {"command": " ".join(shown), "exit": result.returncode, **summarize(output)}


def item_contract(runs, trial, suites):
    audit = test_file_audit()
    default = kernel_default()
    failing = trial["failing_pre_existing_tests"]
    rule = "RecomputeFinal" if failing else "ReuseConfirmed"
    outcome = "versioned" if failing else "default"
    expected_id = ACCOUNTING_IDS[V1 if rule == "RecomputeFinal" else V2]
    decision = {"pass": default == rule and trial["outcome"] == outcome
                and runs["default_accounting"] == expected_id,
                "kernel_default": default, "rule_default": rule, "trial_outcome": trial["outcome"],
                "runs_default_accounting": runs["default_accounting"],
                "trial_failing_pre_existing_tests": len(failing)}
    suites_pass = all(s["exit"] == 0 and s["failed"] == 0 for s in suites)
    return {"pass": audit["pass"] and decision["pass"] and suites_pass, "test_file_audit": audit,
            "decision_rule": decision, "suites": suites, "suites_pass": suites_pass}


def reported(rows, alg04):
    # JVP ratios against the predictions.
    ratios = {}
    for key in REGISTERED_CELLS:
        c1, c2 = rows[key]["arms"][V1].get("counters"), rows[key]["arms"][V2].get("counters")
        if c1 and c2 and c1["jvp_vectors"]:
            ratios["|".join([key[0], key[1], repr(key[2])])] = c2["jvp_vectors"] / c1["jvp_vectors"]
    predictions = []
    for (path, case, rtol) in REGISTERED_CELLS:
        k = "|".join([path, case, repr(rtol)])
        if k not in ratios:
            continue
        if path == "uform_into" and case in PREDICTED_UFORM_INTO:
            lo, hi = PREDICTED_UFORM_INTO[case]
            predictions.append({"cell": [path, case, rtol], "ratio": ratios[k], "predicted": [lo, hi]})
        elif path.startswith("kform"):
            predictions.append({"cell": [path, case, rtol], "ratio": ratios[k], "predicted": list(PREDICTED_KFORM),
                                "within": PREDICTED_KFORM[0] <= ratios[k] <= PREDICTED_KFORM[1]})
    # ALG04 reproduction (C1 cells): uform_into v1 = ALG04 legacy, dup_fix = ALG04 dup_fix (without the log).
    alg04_rows = {(r["case"], r["rtol"]): r for r in alg04["rows"] if r["group"] == "C1"}
    reproduction = []
    for case in SPD07:
        for rtol in RTOLS:
            row = rows[("uform_into", case, rtol)]
            ref = alg04_rows.get((case, rtol))
            entry = {"cell": [case, rtol], "present": ref is not None}
            if ref is not None:
                for ours, theirs in ((V1, "legacy"), ("dup_fix", "dup_fix")):
                    mine = {k: v for k, v in row["arms"][ours].items() if k != "solve_log"}
                    entry[f"{ours}_equals_alg04_{theirs}"] = mine == ref["arms"][theirs]
            reproduction.append(entry)
    # DupFix against v2 on the into path: the zero-iterate exits are the only difference.
    dupfix = []
    for case in SPD07:
        for rtol in RTOLS:
            row = rows[("uform_into", case, rtol)]
            d, a2 = row["arms"]["dup_fix"], row["arms"][V2]
            if d.get("ok") and a2.get("ok"):
                dupfix.append({"cell": [case, rtol], "jvp_dup_fix": d["counters"]["jvp_vectors"],
                               "jvp_v2": a2["counters"]["jvp_vectors"],
                               "diagnostic_v2": a2["counters"]["diagnostic_matvecs"],
                               "trajectory_equal": all(d[k] == a2[k] for k in ("t", "y_last", "attempts", "accepted",
                                                                                "rejected", "solve_log"))})
    return {"jvp_ratio_v2_over_v1": ratios, "predictions": predictions, "alg04_reproduction": reproduction,
            "dup_fix_against_v2": dupfix}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--profile", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    # Evidence validation before any gate: malformed or unbound evidence is INVALID, never PASS/FAIL.
    try:
        validated = validate(args.runs, args.profile)
    except evidence.ValidationError as exc:
        return evidence.emit_invalid(args.output, SCHEMA, exc, {
            "runs": args.runs, "profile": args.profile, "trial": TRIAL, "alg04_runs": ALG04_RUNS})
    runs, profile = validated.docs["runs"], validated.docs["profile"]
    trial, alg04 = validated.docs["trial"], validated.docs["alg04_runs"]
    rows = rows_by_key(runs)
    print("running the kernel contract test ...", flush=True)
    contract = run(CONTRACT)
    contract["pass"] = contract["exit"] == 0 and contract["failed"] == 0 and contract["passed"] > 0
    suites = []
    for name, cmd in SUITES:
        print(f"running {name} ...", flush=True)
        s = run(cmd)
        s["step"] = name
        suites.append(s)
        print(f"  exit {s['exit']}, passed {s['passed']}, failed {s['failed']}", flush=True)
    gate = {
        "1_parity": item_parity(rows, contract),
        "2_exact_accounting": item_accounting(rows),
        "3_no_instruction_regression": item_instructions(profile, rows),
        "4_contract": item_contract(runs, trial, suites),
    }
    verdict = "PASS" if all(item["pass"] for item in gate.values()) else "FAIL"
    head = git("rev-parse", "HEAD").strip()
    report = {
        "schema": SCHEMA,
        "evidence_schema": evidence.VERSION,
        "node": NODE,
        "verdict": verdict,
        "head": head,
        "tree_clean": git("status", "--porcelain", "--untracked-files=no").strip() == "",
        "inputs": {role: {"path": str(p), "sha256": validated.sha256[role]} for role, p in validated.paths.items()},
        "decision_rule_outcome": gate["4_contract"]["decision_rule"],
        "gate": {k: {"pass": v["pass"]} for k, v in gate.items()},
        "items": gate,
        "reported": reported(rows, alg04),
        "claim_scope": "Counted work (WorkCounters) and same-binary callgrind instructions only; no wall-time "
                       "claim; adoption means the registered decision rule, not a global accuracy or speed claim.",
    }
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True, allow_nan=False) + "\n")
    for k, v in gate.items():
        print(f"{k}: {'PASS' if v['pass'] else 'FAIL'}")
    print(f"verdict: {verdict}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
