#!/usr/bin/env python3
"""Checker of research node SP03 (research/sp03_declared_structure_routing_20261010/PREREGISTRATION.md).

Evidence first: the inputs pass the fail-closed rules of ``tools/evidence_schema_v2.py`` (AS03: strict JSON, exact
key and row sets, unique raw keys, canonical finite binary64 hex, typed finite numbers, NATIVE.json bound to its
registered SHA-256) with the SP03 schema defined here (``validate_sp03``): the RUNS cell set, arm sets, record and
routing key sets, references (NATIVE.json values, or a dense 1e-13 run whose decimal and hex states agree),
recorded endpoint errors equal to the errors recomputed from the state bits, the validation cases, the PROFILE
entry set, deterministic callgrind totals with ``ir_per_run = ir_run2 - ir_run1``, and the binding of every
profiled CLI run to the RUNS record of the same arm and cell (attempts, accepted steps and final state bit for
bit). Rejected evidence is the verdict ``INVALID`` (exit 2), never PASS or FAIL.

Gate (registered):

1. Validation: every malformed declaration is refused with its typed error before any integration step
   (declaration or verification stage, no right-hand-side call, no accepted step); every correct declaration is
   accepted and integrates successfully.
2. Parity: ``routed`` equals its target driver called directly in every cell (success, output times, final state,
   attempts, accepted/rejected/internal/clipped steps, driver id; counters equal after subtracting the recorded
   verification charge, and the recorded driver counters equal the target's), the route is the registered one
   (declared-band / declared-dense-small / unstructured-jvp); on the Brusselators ``banded`` equals ``dense``
   (output times, final state, step counts, counters).
3. Strongest applicable arm (callgrind Ir per trajectory): routed / min(dense, dense-colext64) <= 0.60 at n = 400 and
   <= 1.00 at n = 100 and 320; routed / legacy <= 0.05 on every declared Brusselator cell.
4. No hidden O(n^2): log-log slope of routed Ir per accepted step from n = 400 to n = 1000 <= 1.15 (each rtol).
5. Routing overhead: routed Ir per trajectory <= 1.02 x its direct target's (banded, dense or legacy) in every cell.

PASS if every item holds; otherwise FAIL with every number kept. Exit 0 PASS, 1 FAIL, 2 INVALID.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import evidence_schema_v2 as evidence  # noqa: E402  (fail-closed evidence validation, re-audit AS03)

SCHEMA = "vigilode-sp03-routing-check-v1"
RUNS_SCHEMA = "vigilode-sp03-runs-v1"
PROFILE_SCHEMA = "vigilode-sp03-routing-profile-v1"
EVIDENCE_KIND = "SP03"
REPO = Path(__file__).resolve().parent.parent
NATIVE_PATH = "research/stiff_native_benchmark_20261001/NATIVE.json"

RTOLS = (1.0e-6, 1.0e-8)
BRUSSELATOR_CELLS = {"brusselator-1d-50": 100, "brusselator-1d-160": 320, "brusselator-1d-200": 400,
                     "brusselator-1d-500": 1000}
SMALL = {"robertson": 3, "hires": 8, "van-der-pol-mu1000": 2}
NATIVE_PROBLEMS = frozenset(("brusselator-1d-50", "brusselator-1d-200", "robertson", "hires", "van-der-pol-mu1000"))
UNDECLARED = ("brusselator-1d-160", 1.0e-6)
ARMS = {"banded": ("routed", "dense", "dense-colext64", "banded", "legacy"),
        "dense": ("routed", "dense", "dense-colext64"),
        "none": ("routed", "legacy")}
ROUTE = {"banded": ("declared-band", "banded", "banded"),
         "dense": ("declared-dense-small", "dense", "dense"),
         "none": ("unstructured-jvp", "matrix-free", "legacy")}
CLI_ARM = {"rodas5p-routed": "routed", "rodas5p-fast-banded": "banded", "rodas5p-fast": "dense",
           "rodas5p-fast-colext64": "dense-colext64", "rodas5p-mf-legacy": "legacy",
           "rodas5p-routed-undeclared": "routed"}
ERROR_FLOOR = 1.0e-10

COUNTERS = evidence.COUNTERS_FULL | {
    "jacobian_matvecs", "merged_unknown_vector_calls", "phi_weight_underflows", "poly_block_products",
    "poly_vector_products", "poly_coefficient_setups", "poly_coefficient_reuses", "poly_block_allocations",
    "poly_fallbacks", "recycle_update_refreshes"}
RECORD_KEYS = frozenset(("ok", "success", "message", "driver", "t", "y_last", "attempts", "accepted", "rejected",
                         "internal_steps", "output_clipped_steps", "counters", "error"))
FAILED_KEYS = frozenset(("ok", "message"))
ROUTING_KEYS = frozenset(("declared", "dimension", "reason", "driver", "driver_id", "fallback", "verification"))
VERIFICATION_KEYS = frozenset(("lower", "upper", "reference", "vectors", "seed", "tolerance", "relative_mismatch",
                               "band_fills", "band_products", "jvp_products", "dense_jacobian_builds",
                               "dense_products", "charged"))
BANDED_WORK_KEYS = frozenset(("factor_operations", "solve_operations", "stored_slots"))
VALIDATION_KEYS = frozenset(("case", "registered", "problem", "dimension", "declared", "declared_dimension",
                             "band_callback", "expected", "stage", "observed", "message", "rhs_calls",
                             "accepted_steps"))
VALIDATION_REGISTERED = {
    "band-outside-matrix-lower": "BandOutsideMatrix",
    "band-outside-matrix-upper": "BandOutsideMatrix",
    "narrow-band-dense-reference": "BandVerificationMismatch",
    "narrow-band-jvp-reference": "BandVerificationMismatch",
    "missing-band-callback": "MissingBandCallback",
    "dimension-mismatch-band": "DimensionMismatch",
    "dimension-mismatch-dense": "DimensionMismatch",
    "correct-band-dense-reference": "accepted",
    "correct-band-jvp-reference": "accepted",
    "correct-dense-small": "accepted",
}
VALIDATION_REPORTED = {
    "dense-without-explicit-jacobian": "DenseWithoutExplicitJacobian",
    "callback-without-band": "UnexpectedBandCallback",
    "wrong-value-inside-band": "BandVerificationMismatch",
}
PROFILE_KEYS = frozenset(("schema", "label", "commit", "worktree_dirty", "valgrind", "binary_sha256", "environment",
                          "protocol", "entries"))
ENTRY_KEYS = frozenset(("arm", "problem", "rtol", "success", "attempts", "accepted_steps", "rejected_steps",
                        "counters", "banded_work", "final_state_sha256", "ir_run1", "ir_run2", "ir_run1_repeat",
                        "callgrind_deterministic", "ir_per_run", "ir_per_attempt", "ir_per_accepted_step",
                        "categories_by_file", "named_coverage_by_file", "categories_by_function",
                        "named_coverage_by_function", "top_functions", "top_files", "top_lines", "calls",
                        "native_run"))
NATIVE_RUN_KEYS = frozenset(("success", "attempts", "accepted_steps", "rejected_steps", "deterministic",
                             "final_state", "counters_full", "routing"))

THRESHOLDS = {"g3_n400": 0.60, "g3_n100_n320": 1.00, "g3_legacy": 0.05, "g4_slope": 1.15, "g5_overhead": 1.02}


def registered_cells() -> dict[tuple[str, float, str], int]:
    cells = {}
    for problem, n in BRUSSELATOR_CELLS.items():
        for rtol in RTOLS:
            cells[(problem, rtol, "banded")] = n
    cells[(UNDECLARED[0], UNDECLARED[1], "none")] = BRUSSELATOR_CELLS[UNDECLARED[0]]
    for problem, n in SMALL.items():
        for rtol in RTOLS:
            cells[(problem, rtol, "dense")] = n
    return cells


def registered_profile_entries() -> set[tuple[str, str, float]]:
    out = set()
    for problem in BRUSSELATOR_CELLS:
        for rtol in RTOLS:
            for arm in ("rodas5p-routed", "rodas5p-fast-banded", "rodas5p-fast", "rodas5p-fast-colext64",
                        "rodas5p-mf-legacy"):
                out.add((arm, problem, rtol))
    for problem in SMALL:
        for rtol in RTOLS:
            for arm in ("rodas5p-routed", "rodas5p-fast"):
                out.add((arm, problem, rtol))
    out.add(("rodas5p-routed-undeclared", UNDECLARED[0], UNDECLARED[1]))
    return out


def profile_cell(arm: str, problem: str, rtol: float) -> tuple[tuple[str, float, str], str]:
    """The RUNS cell and arm a profile entry describes."""
    if arm == "rodas5p-routed-undeclared":
        return (problem, rtol, "none"), "routed"
    kind = "banded" if problem in BRUSSELATOR_CELLS else "dense"
    return (problem, rtol, kind), CLI_ARM[arm]


def endpoint_error(y: list[float], reference: list[float]) -> float:
    evidence.require_metric_pair(y, reference)
    return max(abs(a - r) / max(abs(r), ERROR_FLOOR) for a, r in zip(y, reference))


def bits(v: float) -> str:
    return struct.pack(">d", v).hex()


# ------------------------------------------------------------------------------------------ evidence
def validate_sp03(runs_path: Path, profile_path: Path, native_path: Path) -> evidence.Evidence:
    """The SP03 schema on the AS03 primitives; raises evidence.ValidationError (INVALID)."""
    docs, digests, paths = evidence._load_all([
        ("runs", runs_path, None),
        ("profile", profile_path, None),
        ("native", native_path, evidence.REGISTERED_SHA256["native"]),
    ])
    c = evidence._Checks()
    runs, profile, native = docs["runs"], docs["profile"], docs["native"]
    native_refs = native.get("references", {}) if isinstance(native, dict) else {}
    cells_registered = registered_cells()
    problems = {k[0]: n for k, n in cells_registered.items()}

    # RUNS header.
    if not c.keys("runs", runs, ("schema", "error_metric", "initial_step", "max_attempts", "band_verification",
                                 "references", "cells", "validation")):
        c.done()
    c.equal("runs.schema", runs["schema"], RUNS_SCHEMA)
    c.str_("runs.error_metric", runs["error_metric"])
    c.equal("runs.initial_step", runs["initial_step"], 1.0e-6)
    c.equal("runs.max_attempts", runs["max_attempts"], 1_000_000)
    bv = runs["band_verification"]
    if c.keys("runs.band_verification", bv, ("vectors", "tolerance", "seed")):
        c.equal("runs.band_verification.vectors", bv["vectors"], 2)
        c.equal("runs.band_verification.tolerance", bv["tolerance"], 1.0e-12)
        c.int_("runs.band_verification.seed", bv["seed"])

    # References.
    references: dict[str, list[float]] = {}
    refs = runs["references"]
    if c.keys("runs.references", refs, problems):
        for problem, ref in refs.items():
            path, n = f"runs.references.{problem}", problems[problem]
            if problem in NATIVE_PROBLEMS:
                if not c.keys(path, ref, ("kind", "path", "y")):
                    continue
                c.equal(f"{path}.kind", ref["kind"], "native-json")
                c.equal(f"{path}.path", ref["path"], NATIVE_PATH)
                y = c.float_vector(f"{path}.y", ref["y"], n)
                expected = native_refs.get(problem, {}).get("final_state") if isinstance(native_refs, dict) else None
                if y is not None and y != expected:
                    c.fail(f"{path}.y", "differs from the registered NATIVE.json reference")
                if y is not None:
                    references[problem] = y
            else:
                if not c.keys(path, ref, ("kind", "y", "y_hex", "attempts", "accepted")):
                    continue
                c.equal(f"{path}.kind", ref["kind"], "dense-fast-rtol-1e-13")
                y = c.float_vector(f"{path}.y", ref["y"], n)
                yh = c.hex_vector(f"{path}.y_hex", ref["y_hex"], n)
                c.int_(f"{path}.attempts", ref["attempts"], minimum=1)
                c.int_(f"{path}.accepted", ref["accepted"], minimum=1)
                if y is not None and yh is not None:
                    if [bits(v) for v in y] != [bits(v) for v in yh]:
                        c.fail(path, "decimal and hex reference states differ")
                    else:
                        references[problem] = y

    # Cells.
    cells: dict[tuple[str, float, str], dict] = {}
    if c.list_("runs.cells", runs["cells"], nonempty=True):
        for i, cell in enumerate(runs["cells"]):
            path = f"runs.cells[{i}]"
            if not c.keys(path, cell, ("problem", "dimension", "rtol", "atol", "declared", "arms")):
                continue
            if not (c.str_(f"{path}.problem", cell["problem"]) and c.float_(f"{path}.rtol", cell["rtol"], positive=True)
                    and c.str_(f"{path}.declared", cell["declared"])):
                continue
            key = (cell["problem"], cell["rtol"], cell["declared"])
            if key in cells:
                c.fail(path, f"duplicate cell {list(key)}")
                continue
            if key not in cells_registered:
                c.fail(path, f"unregistered cell {list(key)}")
                continue
            cells[key] = cell
            n = cells_registered[key]
            c.equal(f"{path}.dimension", cell["dimension"], n)
            c.float_(f"{path}.atol", cell["atol"], positive=True)
            if not c.keys(f"{path}.arms", cell["arms"], ARMS[key[2]]):
                continue
            for arm, rec in cell["arms"].items():
                _record(c, f"{path}.arms.{arm}", rec, arm, key[2], n, references.get(key[0]))
    missing = sorted(set(cells_registered) - set(cells))
    if missing:
        c.fail("runs.cells", f"missing registered cells {missing}")

    # Validation cases.
    expected_cases = {**VALIDATION_REGISTERED, **VALIDATION_REPORTED}
    seen = set()
    if c.list_("runs.validation", runs["validation"], nonempty=True):
        for i, row in enumerate(runs["validation"]):
            path = f"runs.validation[{i}]"
            accepted = isinstance(row, dict) and row.get("observed") == "accepted"
            keys = VALIDATION_KEYS | ({"routing"} if accepted else set())
            if not c.keys(path, row, keys):
                continue
            name = row["case"]
            if name in seen:
                c.fail(path, f"duplicate case {name!r}")
            seen.add(name)
            if name not in expected_cases:
                c.fail(path, f"unregistered case {name!r}")
                continue
            c.equal(f"{path}.registered", row["registered"], name in VALIDATION_REGISTERED)
            c.equal(f"{path}.expected", row["expected"], expected_cases[name])
            c.str_(f"{path}.observed", row["observed"])
            c.str_(f"{path}.stage", row["stage"])
            c.int_(f"{path}.rhs_calls", row["rhs_calls"])
            c.int_(f"{path}.accepted_steps", row["accepted_steps"])
            c.int_(f"{path}.dimension", row["dimension"], minimum=1)
            c.int_(f"{path}.declared_dimension", row["declared_dimension"])
            c.bool_(f"{path}.band_callback", row["band_callback"])
            if accepted:
                _routing(c, f"{path}.routing", row["routing"])
        if set(expected_cases) - seen:
            c.fail("runs.validation", f"missing cases {sorted(set(expected_cases) - seen)}")

    # PROFILE.
    entries: dict[tuple[str, str, float], dict] = {}
    if c.keys("profile", profile, PROFILE_KEYS):
        c.equal("profile.schema", profile["schema"], PROFILE_SCHEMA)
        c.equal("profile.worktree_dirty", profile["worktree_dirty"], False)
        c.equal("profile.environment", profile["environment"], {"RAYON_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1"})
        c.str_("profile.commit", profile["commit"])
        c.str_("profile.binary_sha256", profile["binary_sha256"])
        if c.list_("profile.entries", profile["entries"], nonempty=True):
            for i, e in enumerate(profile["entries"]):
                path = f"profile.entries[{i}]"
                if not c.keys(path, e, ENTRY_KEYS):
                    continue
                if not (c.str_(f"{path}.arm", e["arm"]) and c.str_(f"{path}.problem", e["problem"])
                        and c.float_(f"{path}.rtol", e["rtol"], positive=True)):
                    continue
                key = (e["arm"], e["problem"], e["rtol"])
                if key in entries:
                    c.fail(path, f"duplicate entry {list(key)}")
                    continue
                entries[key] = e
                _entry(c, path, e, cells)
        expected_entries = registered_profile_entries()
        if set(entries) != expected_entries:
            c.fail("profile.entries", f"entry set differs (missing {sorted(expected_entries - set(entries))}, "
                                      f"unexpected {sorted(set(entries) - expected_entries)})")
    c.done()
    return evidence.Evidence(EVIDENCE_KIND, {**docs, "cells": cells, "entries": entries,
                                             "references_y": references}, digests, paths)


def _counters(c, path, value) -> None:
    if c.keys(path, value, COUNTERS):
        for k, v in value.items():
            c.int_(f"{path}.{k}", v)


def _routing(c, path, r) -> None:
    if not c.keys(path, r, ROUTING_KEYS):
        return
    c.int_(f"{path}.dimension", r["dimension"], minimum=1)
    for k in ("reason", "driver", "driver_id"):
        c.str_(f"{path}.{k}", r[k])
    if c.dict_(f"{path}.declared", r["declared"]):
        kind = r["declared"].get("kind")
        keys = ("kind", "lower", "upper") if kind == "banded" else ("kind",)
        c.keys(f"{path}.declared", r["declared"], keys)
    if r["fallback"] is not None:
        c.dict_(f"{path}.fallback", r["fallback"])
    v = r["verification"]
    if v is not None and c.keys(f"{path}.verification", v, VERIFICATION_KEYS):
        for k in ("lower", "upper", "vectors", "seed", "band_fills", "band_products", "jvp_products",
                  "dense_jacobian_builds", "dense_products"):
            c.int_(f"{path}.verification.{k}", v[k])
        c.str_(f"{path}.verification.reference", v["reference"])
        c.float_(f"{path}.verification.tolerance", v["tolerance"], positive=True)
        if c.list_(f"{path}.verification.relative_mismatch", v["relative_mismatch"], nonempty=True):
            for j, m in enumerate(v["relative_mismatch"]):
                c.float_(f"{path}.verification.relative_mismatch[{j}]", m, minimum=0.0)
        _counters(c, f"{path}.verification.charged", v["charged"])


def _record(c, path, rec, arm, kind, n, reference) -> None:
    if not c.dict_(path, rec):
        return
    if rec.get("ok") is False:
        c.keys(path, rec, FAILED_KEYS)
        c.str_(f"{path}.message", rec.get("message"))
        return
    keys = set(RECORD_KEYS)
    if arm == "routed":
        keys |= {"routing", "driver_counters"}
        if kind == "banded":
            keys.add("banded_work")
    if arm == "banded":
        keys.add("banded_work")
    if not c.keys(path, rec, keys):
        return
    c.equal(f"{path}.ok", rec["ok"], True)
    c.bool_(f"{path}.success", rec["success"])
    c.str_(f"{path}.message", rec["message"])
    c.str_(f"{path}.driver", rec["driver"])
    c.hex_vector(f"{path}.t", rec["t"])
    y = c.hex_vector(f"{path}.y_last", rec["y_last"], n)
    for k in ("attempts", "accepted", "rejected", "internal_steps", "output_clipped_steps"):
        c.int_(f"{path}.{k}", rec[k])
    _counters(c, f"{path}.counters", rec["counters"])
    if c.float_(f"{path}.error", rec["error"], minimum=0.0) and y is not None and reference is not None:
        if endpoint_error(y, reference) != rec["error"]:
            c.fail(f"{path}.error", f"recorded {rec['error']!r} != recomputed {endpoint_error(y, reference)!r}")
    if "banded_work" in keys and c.keys(f"{path}.banded_work", rec["banded_work"], BANDED_WORK_KEYS):
        for k, v in rec["banded_work"].items():
            c.int_(f"{path}.banded_work.{k}", v)
    if arm == "routed":
        _routing(c, f"{path}.routing", rec["routing"])
        _counters(c, f"{path}.driver_counters", rec["driver_counters"])


def _entry(c, path, e, cells) -> None:
    c.equal(f"{path}.success", e["success"], True)
    for k in ("attempts", "accepted_steps", "rejected_steps"):
        c.int_(f"{path}.{k}", e[k], minimum=0)
    for k in ("ir_run1", "ir_run2", "ir_run1_repeat", "ir_per_run"):
        c.int_(f"{path}.{k}", e[k], minimum=1)
    c.equal(f"{path}.callgrind_deterministic", e["callgrind_deterministic"], True)
    if all(type(e[k]) is int for k in ("ir_run1", "ir_run2", "ir_run1_repeat", "ir_per_run")):
        if e["ir_run1"] != e["ir_run1_repeat"]:
            c.fail(path, "1-repetition callgrind totals differ")
        if e["ir_per_run"] != e["ir_run2"] - e["ir_run1"]:
            c.fail(f"{path}.ir_per_run", "is not ir_run2 - ir_run1")
    c.float_(f"{path}.ir_per_accepted_step", e["ir_per_accepted_step"], positive=True)
    nr = e["native_run"]
    if not c.keys(f"{path}.native_run", nr, NATIVE_RUN_KEYS):
        return
    c.equal(f"{path}.native_run.success", nr["success"], True)
    c.equal(f"{path}.native_run.deterministic", nr["deterministic"], True)
    for k in ("attempts", "accepted_steps", "rejected_steps"):
        if nr[k] != e[k]:
            c.fail(f"{path}.native_run.{k}", f"{nr[k]!r} != profiled {e[k]!r}")
    cell_key, arm = profile_cell(e["arm"], e["problem"], e["rtol"])
    cell = cells.get(cell_key)
    if cell is None:
        c.fail(path, f"no RUNS cell {list(cell_key)}")
        return
    n = cell["dimension"]
    y = c.float_vector(f"{path}.native_run.final_state", nr["final_state"], n)
    if y is not None and hashlib.sha256(json.dumps(nr["final_state"]).encode()).hexdigest() != e["final_state_sha256"]:
        c.fail(f"{path}.final_state_sha256", "is not the hash of the native run's final state")
    rec = cell.get("arms", {}).get(arm)
    if not isinstance(rec, dict) or rec.get("ok") is not True:
        c.fail(path, f"RUNS has no successful {arm} record in {list(cell_key)}")
        return
    # Binding of the CLI run to the library run.
    if rec.get("attempts") != e["attempts"] or rec.get("accepted") != e["accepted_steps"]:
        c.fail(path, f"CLI attempts/accepted {e['attempts']}/{e['accepted_steps']} != RUNS "
                     f"{rec.get('attempts')}/{rec.get('accepted')}")
    if y is not None and isinstance(rec.get("y_last"), list) and [bits(v) for v in y] != rec["y_last"]:
        c.fail(path, "CLI final state differs from the RUNS state bit for bit")
    if arm == "routed":
        if not isinstance(nr["routing"], dict):
            c.fail(f"{path}.native_run.routing", "missing routing record")
        elif nr["routing"].get("reason") != rec.get("routing", {}).get("reason"):
            c.fail(f"{path}.native_run.routing.reason", "differs from the RUNS route")


# ---------------------------------------------------------------------------------------------- gates
def subtract(a: dict, b: dict) -> dict:
    return {k: a[k] - b.get(k, 0) for k in a}


def gate_validation(runs) -> dict:
    rows = []
    ok = True
    for row in runs["validation"]:
        malformed = row["expected"] != "accepted"
        if malformed:
            holds = (row["observed"] == row["expected"] and row["stage"] in ("declaration", "verification")
                     and row["rhs_calls"] == 0 and row["accepted_steps"] == 0)
        else:
            holds = (row["observed"] == "accepted" and row["stage"] == "integration" and row["message"] == "success"
                     and row["accepted_steps"] > 0)
        ok &= holds
        rows.append({"case": row["case"], "registered": row["registered"], "expected": row["expected"],
                     "observed": row["observed"], "stage": row["stage"], "rhs_calls": row["rhs_calls"],
                     "message": row["message"], "holds": holds})
    return {"item": "validation: malformed declarations refused with their typed error before any step; correct "
                    "declarations accepted", "cases": rows, "pass": ok}


PARITY_KEYS = ("success", "t", "y_last", "attempts", "accepted", "rejected", "internal_steps", "output_clipped_steps",
               "driver")


def gate_parity(cells) -> dict:
    rows = []
    ok = True
    for key in sorted(cells, key=lambda k: (k[2], k[0], k[1])):
        arms = cells[key]["arms"]
        reason, driver, target_arm = ROUTE[key[2]]
        routed, target = arms["routed"], arms[target_arm]
        diffs = []
        if routed.get("ok") is not True or target.get("ok") is not True:
            diffs.append("a run failed")
        else:
            diffs += [k for k in PARITY_KEYS if routed[k] != target[k]]
            v = routed["routing"]["verification"]
            charged = v["charged"] if v is not None else {}
            if subtract(routed["counters"], charged) != target["counters"]:
                diffs.append("counters minus verification charge")
            if routed["driver_counters"] != target["counters"]:
                diffs.append("driver_counters")
            if routed["routing"]["reason"] != reason or routed["routing"]["driver"] != driver:
                diffs.append(f"route {routed['routing']['reason']}/{routed['routing']['driver']}")
            if (key[2] == "banded") != (v is not None):
                diffs.append("verification record")
        band_dense = None
        if key[2] == "banded":
            b, d = arms["banded"], arms["dense"]
            if b.get("ok") is not True or d.get("ok") is not True:
                band_dense = ["a run failed"]
            else:
                band_dense = [k for k in ("success", "t", "y_last", "attempts", "accepted", "rejected",
                                          "internal_steps", "output_clipped_steps", "counters") if b[k] != d[k]]
        holds = not diffs and not band_dense
        ok &= holds
        rows.append({"problem": key[0], "rtol": key[1], "declared": key[2], "target": target_arm,
                     "routed_vs_target_differences": diffs, "banded_vs_dense_differences": band_dense,
                     "holds": holds})
    return {"item": "parity: routed bitwise equal to its target driver (counters after the verification charge); "
                    "banded bitwise equal to dense on the Brusselators", "cells": rows, "pass": ok}


def ir(entries, arm, problem, rtol, field="ir_per_run"):
    return entries[(arm, problem, rtol)][field]


def gate_strongest(entries) -> dict:
    rows = []
    ok = True
    for problem, n in BRUSSELATOR_CELLS.items():
        for rtol in RTOLS:
            routed = ir(entries, "rodas5p-routed", problem, rtol)
            dense = ir(entries, "rodas5p-fast", problem, rtol)
            colext = ir(entries, "rodas5p-fast-colext64", problem, rtol)
            legacy = ir(entries, "rodas5p-mf-legacy", problem, rtol)
            direct = routed / min(dense, colext)
            vs_legacy = routed / legacy
            bound = THRESHOLDS["g3_n400"] if n == 400 else THRESHOLDS["g3_n100_n320"] if n in (100, 320) else None
            holds_direct = True if bound is None else direct <= bound
            holds_legacy = vs_legacy <= THRESHOLDS["g3_legacy"]
            ok &= holds_direct and holds_legacy
            rows.append({"problem": problem, "n": n, "rtol": rtol, "ir_routed": routed, "ir_dense": dense,
                         "ir_dense_colext64": colext, "ir_legacy": legacy,
                         "routed_over_min_dense": direct, "bound_min_dense": bound,
                         "routed_over_legacy": vs_legacy, "bound_legacy": THRESHOLDS["g3_legacy"],
                         "holds": holds_direct and holds_legacy})
    return {"item": "strongest applicable arm: routed/min(dense, colext64) <= 0.60 at n = 400, <= 1.00 at n = 100 "
                    "and 320; routed/legacy <= 0.05 on every Brusselator cell (Ir per trajectory)",
            "cells": rows, "pass": ok}


def gate_slope(entries) -> dict:
    rows = []
    ok = True
    for rtol in RTOLS:
        a = ir(entries, "rodas5p-routed", "brusselator-1d-200", rtol, "ir_per_accepted_step")
        b = ir(entries, "rodas5p-routed", "brusselator-1d-500", rtol, "ir_per_accepted_step")
        slope = math.log(b / a) / math.log(1000 / 400)
        holds = slope <= THRESHOLDS["g4_slope"]
        ok &= holds
        rows.append({"rtol": rtol, "ir_per_accepted_step_n400": a, "ir_per_accepted_step_n1000": b,
                     "slope": slope, "bound": THRESHOLDS["g4_slope"], "holds": holds})
    return {"item": "no hidden O(n^2): routed Ir per accepted step slope n = 400 -> 1000 <= 1.15", "rtols": rows,
            "pass": ok}


def gate_overhead(entries) -> dict:
    rows = []
    ok = True
    pairs = []
    for problem in BRUSSELATOR_CELLS:
        for rtol in RTOLS:
            pairs.append(("rodas5p-routed", "rodas5p-fast-banded", problem, rtol, "banded"))
    for problem in SMALL:
        for rtol in RTOLS:
            pairs.append(("rodas5p-routed", "rodas5p-fast", problem, rtol, "dense"))
    pairs.append(("rodas5p-routed-undeclared", "rodas5p-mf-legacy", UNDECLARED[0], UNDECLARED[1], "none"))
    for arm, target, problem, rtol, kind in pairs:
        r, t = ir(entries, arm, problem, rtol), ir(entries, target, problem, rtol)
        ratio = r / t
        holds = ratio <= THRESHOLDS["g5_overhead"]
        ok &= holds
        rows.append({"problem": problem, "rtol": rtol, "declared": kind, "routed_arm": arm, "target_arm": target,
                     "ir_routed": r, "ir_target": t, "difference": r - t, "ratio": ratio,
                     "bound": THRESHOLDS["g5_overhead"], "holds": holds})
    return {"item": "routing overhead: routed Ir per trajectory <= 1.02 x its direct target, verification included",
            "cells": rows, "pass": ok}


def reported(cells, entries) -> dict:
    routes = []
    errors = []
    for key in sorted(cells, key=lambda k: (k[2], k[0], k[1])):
        arms = cells[key]["arms"]
        r = arms["routed"]
        if r.get("ok"):
            v = r["routing"]["verification"]
            routes.append({"problem": key[0], "rtol": key[1], "declared": key[2],
                           "reason": r["routing"]["reason"], "driver": r["routing"]["driver"],
                           "driver_id": r["routing"]["driver_id"], "fallback": r["routing"]["fallback"],
                           "verification": None if v is None else {
                               "reference": v["reference"], "relative_mismatch": v["relative_mismatch"],
                               "band_fills": v["band_fills"], "band_products": v["band_products"],
                               "jvp_products": v["jvp_products"], "dense_jacobian_builds": v["dense_jacobian_builds"],
                               "dense_products": v["dense_products"],
                               "charged_nonzero": {k: x for k, x in v["charged"].items() if x}}})
        errors.append({"problem": key[0], "rtol": key[1], "declared": key[2],
                       "error": {a: (rec["error"] if rec.get("ok") else None) for a, rec in arms.items()},
                       "accepted": {a: (rec["accepted"] if rec.get("ok") else None) for a, rec in arms.items()}})
    verification_ir = []
    for problem in BRUSSELATOR_CELLS:
        for rtol in RTOLS:
            r, b = ir(entries, "rodas5p-routed", problem, rtol), ir(entries, "rodas5p-fast-banded", problem, rtol)
            verification_ir.append({"problem": problem, "rtol": rtol, "ir_routed_minus_banded": r - b,
                                    "fraction_of_banded": (r - b) / b})
    small = []
    for problem in SMALL:
        for rtol in RTOLS:
            r, d = ir(entries, "rodas5p-routed", problem, rtol), ir(entries, "rodas5p-fast", problem, rtol)
            small.append({"problem": problem, "rtol": rtol, "ir_routed": r, "ir_dense": d, "difference": r - d,
                          "ratio": r / d})
    per_attempt = [{"arm": a, "problem": p, "rtol": t, "attempts": e["attempts"], "accepted": e["accepted_steps"],
                    "ir_per_run": e["ir_per_run"], "ir_per_attempt": e["ir_per_attempt"],
                    "ir_per_accepted_step": e["ir_per_accepted_step"]}
                   for (a, p, t), e in sorted(entries.items())]
    return {"routing": routes, "endpoint_errors": errors, "verification_and_routing_ir": verification_ir,
            "small_problem_cells": small, "profile_table": per_attempt}


# ----------------------------------------------------------------------------------------------- main
def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--profile", required=True, type=Path)
    parser.add_argument("--native", type=Path, default=REPO / NATIVE_PATH)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    inputs = {"runs": args.runs, "profile": args.profile, "native": args.native}
    try:
        validated = validate_sp03(args.runs, args.profile, args.native)
    except evidence.ValidationError as exc:
        return evidence.emit_invalid(args.output, SCHEMA, exc, inputs)
    runs = validated.docs["runs"]
    cells = validated.docs["cells"]
    entries = validated.docs["entries"]
    gates = {
        "1_validation": gate_validation(runs),
        "2_parity": gate_parity(cells),
        "3_strongest_arm": gate_strongest(entries),
        "4_no_hidden_quadratic": gate_slope(entries),
        "5_routing_overhead": gate_overhead(entries),
    }
    verdict = "PASS" if all(g["pass"] for g in gates.values()) else "FAIL"
    report = {
        "schema": SCHEMA,
        "evidence_schema": evidence.VERSION,
        "evidence_kind": EVIDENCE_KIND,
        "verdict": verdict,
        "inputs": {role: {"path": str(path), "sha256": validated.sha256[role]} for role, path in inputs.items()},
        "profile_commit": validated.docs["profile"]["commit"],
        "binary_sha256": validated.docs["profile"]["binary_sha256"],
        "thresholds": THRESHOLDS,
        "gate": gates,
        "reported": reported(cells, entries),
        "claim_scope": "Same-binary callgrind instructions and counted work of the opt-in router on the registered "
                       "cells; no wall-time claim; the banded path is not a general sparse LU.",
    }
    args.output.write_text(json.dumps(report, indent=1, allow_nan=False) + "\n")
    print(f"verdict: {verdict}")
    for name, g in gates.items():
        print(f"  {name}: {'holds' if g['pass'] else 'FAILS'}")
    return 0 if verdict == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
