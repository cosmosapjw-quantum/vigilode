#!/usr/bin/env python3
"""Synthetic tests for tools/py03_single_backend_router_check.py (research node PY03).

Every fixture is synthetic: a cases document built here for a reduced grid (n = 8, rho in {4, 64}, h = 0.05, both
inputs, total budget 1e-6: eight grid cases), the five boundary cases and one ForceLaguerreFirst case per family,
with the registered constructions of the diagonal matrices and the inputs, small symmetric "rotated" matrices,
results rounded from the 50-digit reference and consistent costs and counters. No Rust code runs and no committed
research file is read or written. The grid, the niche minimum and the git provenance hooks are patched in-process:
- a valid fixture PASSes (exit 0), and FAILs as "niche empty" under the registered minimum 6;
- each validity condition violated on its own -> INVALID (exit 2);
- each gate item violated on its own -> FAIL (exit 1) on that item only.

Run: python3 -m unittest discover -s tools -p test_py03_single_backend_router_check.py
"""

from __future__ import annotations

import copy
import json
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import py03_single_backend_router_check as pc  # noqa: E402

COMMIT = "a" * 40
CHECKER_BYTES = (TOOLS / "py03_single_backend_router_check.py").read_bytes()
REDUCED = {"GRID_N": (8,), "GRID_RHO": (4.0, 64.0), "GRID_H": (0.05,), "GRID_BUDGET": (1e-6,)}
NICHE_LABELS = ("diagonal-n8-rho4-h0.05-distinct-b1e-6", "rotated-n8-rho64-h0.05-same-b1e-6")


def hx(x: float) -> str:
    return struct.pack(">d", x).hex()


def zero_cost() -> dict:
    return {k: 0 for k in pc.COST_KEYS}


def zero_work() -> dict:
    return {k: 0 for k in sorted(pc.WORK_KEYS)}


def add(a: dict, b: dict) -> dict:
    return {k: a[k] + b[k] for k in a}


def rotated(n: int, rho: float, seed: int) -> list[list[float]]:
    """A small symmetric negative definite matrix (the checker cannot rebuild the Givens construction)."""
    vals = pc.splitmix(seed, n * n)
    a = [[0.0] * n for _ in range(n)]
    for i in range(n):
        a[i][i] = -rho * (0.3 + 0.6 * (vals[i * n + i] + 1.0) / 2.0)
        for j in range(i):
            v = 0.01 * rho * vals[i * n + j]
            a[i][j] = v
            a[j][i] = v
    return a


def plan(basis: str, degree: int, width: int, tails: int = 7, branch: str = "recurrence") -> dict:
    cost = zero_cost()
    cost["plan_tail_evaluations"] = tails
    lag = basis == "Laguerre"
    return {"basis": basis, "error": None, "cost": cost,
            "plan": {"branch": branch, "degree": degree, "width": width,
                     "laguerre_scale": hx(16.0) if lag else None, "beta": hx(4.0) if lag else None,
                     "laguerre_extent": hx(16.0) if lag else None,
                     "predicted_vector_products": degree * width, "predicted_block_products": degree,
                     "tail_evaluations": tails, "normalization_shift": 0, "coefficients_cached": False}}


def failed_plan(basis: str) -> dict:
    return {"basis": basis, "plan": None, "error": "truncation budget must be positive", "cost": zero_cost()}


def report(basis: str, degree: int, fused: list[str], total: str | None, *, hit: bool = False,
           execution: str = pc.EXEC_CERTIFIED, verified: bool = True, branch: str = "recurrence") -> dict:
    return {"basis": basis, "branch": branch, "execution": execution, "evidence_verified": verified,
            "degree": degree, "fused": fused,
            "total_error_is_estimate": basis == "Laguerre" or not verified or execution != pc.EXEC_CERTIFIED,
            "candidate_total": total, "coefficient_cache_hit": hit}


def execution(basis: str, degree: int, width: int, fused: list[str], bound: str | None, *, hit: bool = False,
              admitted: bool = True, envelope_ops: int = 10, branch: str = "recurrence") -> dict:
    cost, work = zero_cost(), zero_work()
    cost["executed_backends"] = 1
    cost["failed_executions"] = 0 if admitted else 1
    cost["vector_products"] = degree * width
    cost["block_products"] = degree
    work["poly_vector_products"] = degree * width
    work["poly_block_products"] = degree
    if branch == "recurrence":
        work["poly_block_allocations"] = 3
        if hit:
            cost["coefficient_cache_hits"] = 1
            work["poly_coefficient_reuses"] = 1
        else:
            cost["coefficient_setups"] = 1
            cost["coefficient_series_terms"] = 100
            work["poly_coefficient_setups"] = 1
    if basis == "Chebyshev" and branch == "recurrence":
        cost["chebyshev_propagation_terms"] = 5 * degree * (degree + 1) // 2
    elif basis == "Laguerre":
        cost["envelope_interval_operations"] = 0 if hit else envelope_ops
        cost["envelope_cache_hits" if hit else "envelope_cache_misses"] = 5
        cost["adjoint_sum_terms"] = 5 * degree
    total = bound if bound is not None else hx(1.0)
    rep = report(basis, degree, fused, total, hit=hit, branch=branch)
    adm = ({"admitted": True, "bound": bound, "reason": None} if admitted
           else {"admitted": False, "bound": None, "reason": "TOTAL_ERROR_ABOVE_BUDGET: synthetic"})
    return {"basis": basis, "executed": True, "degree": degree, "admission": adm, "cost": cost, "work": work,
            "report": rep, "parity": {"report_bitwise_equal": True, "counters_equal": True,
                                      "replay_fused": list(fused), "replay_candidate_total": total}}


def unexecuted(basis: str) -> dict:
    cost = zero_cost()
    cost["failed_executions"] = 1
    return {"basis": basis, "executed": False, "degree": None,
            "admission": {"admitted": False, "bound": None, "reason": "PY03_ROUTER_PLAN_FAILED: synthetic"},
            "cost": cost, "work": zero_work(), "report": None, "parity": None}


def preflight(preselected: str | None, reason: str | None, cost: dict | None = None,
              work: dict | None = None, table: bool = False) -> dict:
    return {"preselected": preselected, "abstain_reason": reason, "laguerre_table_used": table,
            "vector_independent_terms": hx(1e-12) if table else None, "cost": cost or zero_cost(),
            "work": work or zero_work()}


def router_record(arm: str, state: str, plans: list, pf: dict, executions: list) -> dict:
    cost = zero_cost()
    for part in [p["cost"] for p in plans] + [pf["cost"]] + [e["cost"] for e in executions]:
        cost = add(cost, part)
    work = pf["work"]
    for e in executions:
        work = add(work, e["work"])
    admitted = [e for e in executions if e["admission"]["admitted"]]
    if admitted:
        e = admitted[-1]
        choice, fused, bound, reasons = e["basis"], e["report"]["fused"], e["admission"]["bound"], None
    else:
        choice, fused, bound = "Fallback", None, None
        reasons = {"chebyshev": "synthetic rejection", "laguerre": "synthetic rejection"}
    return {"arm": arm, "cache_state": state, "policy": pc.POLICY[arm], "choice": choice,
            "fallback_reasons": reasons, "fused": fused, "bound": bound, "plans": plans, "preflight": pf,
            "executions": executions, "cost": cost, "caller_counters": work,
            "caches_before": {"coefficient_tables": 0 if state == "cold" else 1, "envelopes": 0},
            "caches_after": {"coefficient_tables": 1, "envelopes": 0}}


def pp08_record(state: str, attempts: list) -> dict:
    work = zero_work()
    for a in attempts:
        work = add(work, a["work"])
    admitted = [a for a in attempts if a["admission"]["admitted"]]
    if admitted:
        best = admitted[0]
        choice, fused, bound, reasons = best["basis"], best["report"]["fused"], best["admission"]["bound"], None
    else:
        choice, fused, bound = "Fallback", None, None
        reasons = {"chebyshev": "synthetic", "laguerre": "synthetic"}
    return {"arm": "pp08", "cache_state": state, "choice": choice, "fallback_reasons": reasons, "fused": fused,
            "bound": bound, "attempts": attempts, "caller_counters": work}


def attempt(ex: dict) -> dict:
    return {"basis": ex["basis"], "degree": ex["degree"], "admission": ex["admission"],
            "vector_products": ex["cost"]["vector_products"], "work": ex["work"], "report": ex["report"]}


class Builder:
    def __init__(self):
        self.patches = [mock.patch.object(pc, k, v) for k, v in REDUCED.items()]

    def __enter__(self):
        for p in self.patches:
            p.start()
        return self

    def __exit__(self, *exc):
        for p in self.patches:
            p.stop()

    @staticmethod
    def fused(a, h, w) -> list[str]:
        return [hx(float(x)) for x in pc.exact_target(a, h, w, pc.DPS)]

    def case(self, label, group, family, n, rho, seed, input_seed, a, h, kind, budget, *, enclosure=None,
             niche=False, force=False, declared=False, zero=False, scalar=False, above=False, timing=False):
        w = pc.registered_w(kind, n, input_seed)
        width = 5 if kind == "distinct" else 1
        fused = self.fused(a, h, w)
        bound = hx(min(budget, 1e-9) * 0.5) if budget > 0 else None
        recs = []
        for state in pc.STATES:
            cold = state == "cold"
            if declared:
                reason = "PY03_ROUTER_ENCLOSURE_NOT_VERIFIED: synthetic"
                for arm in ("single", "cheb"):
                    plans = [plan("Chebyshev", 6, width)] + ([] if arm == "cheb" else [plan("Laguerre", 8, width)])
                    recs.append(router_record(arm, state, plans, preflight(None, reason), []))
                ats = [execution(b, d, width, fused, None, admitted=False) for b, d in
                       (("Chebyshev", 6), ("Laguerre", 8))]
                for at in ats:
                    at["report"]["evidence_verified"] = False
                    at["report"]["total_error_is_estimate"] = True
                recs.append(pp08_record(state, [attempt(x) for x in ats]))
                continue
            if zero:
                for arm in ("single", "cheb"):
                    bases = ["Chebyshev"] if arm == "cheb" else ["Chebyshev", "Laguerre"]
                    reason = None if arm == "cheb" else "PY03_PREFLIGHT_LAGUERRE_PLAN_FAILED: synthetic"
                    recs.append(router_record(arm, state, [failed_plan(b) for b in bases],
                                              preflight("Chebyshev", reason), [unexecuted(b) for b in bases]))
                ats = []
                for b in pc.BASES:
                    u = unexecuted(b)
                    ats.append({"basis": b, "degree": None, "admission": u["admission"], "vector_products": 0,
                                "work": zero_work(), "report": None})
                recs.append(pp08_record(state, ats))
                continue
            if scalar:
                plans = [plan("Chebyshev", 0, width, 0, "scalar"), plan("Laguerre", 0, width, 0, "scalar")]
                ex = execution("Chebyshev", 0, width, fused, bound, branch="scalar")
                ex["report"]["branch"] = "scalar"
                recs.append(router_record("single", state, plans,
                                          preflight("Chebyshev", "PY03_PREFLIGHT_PRODUCTS_NOT_FEWER: 0 >= 0"), [ex]))
                recs.append(router_record("cheb", state, plans[:1], preflight("Chebyshev", None),
                                          [copy.deepcopy(ex)]))
                lag = execution("Laguerre", 0, width, fused, bound, branch="scalar")
                lag["report"]["branch"] = "scalar"
                lag["report"]["total_error_is_estimate"] = False
                recs.append(pp08_record(state, [attempt(ex), attempt(lag)]))
                continue
            lag_degree = 140 if above else (4 if niche else 8)
            cheb_ex = execution("Chebyshev", 6, width, fused, bound, hit=not cold)
            if niche:
                pf_cost, pf_work = zero_cost(), zero_work()
                if cold:
                    pf_cost["coefficient_setups"], pf_cost["coefficient_series_terms"] = 1, 50
                    pf_work["poly_coefficient_setups"] = 1
                else:
                    pf_cost["coefficient_cache_hits"] = 1
                    pf_work["poly_coefficient_reuses"] = 1
                lag_ex = execution("Laguerre", 4, width, fused, bound, hit=True)
                recs.append(router_record("single", state,
                                          [plan("Chebyshev", 6, width), plan("Laguerre", 4, width)],
                                          preflight("Laguerre", None, pf_cost, pf_work, table=True), [lag_ex]))
            else:
                recs.append(router_record("single", state,
                                          [plan("Chebyshev", 6, width), plan("Laguerre", lag_degree, width)],
                                          preflight("Chebyshev", "PY03_PREFLIGHT_PRODUCTS_NOT_FEWER: synthetic"),
                                          [cheb_ex]))
            recs.append(router_record("cheb", state, [plan("Chebyshev", 6, width)], preflight("Chebyshev", None),
                                      [copy.deepcopy(cheb_ex)]))
            if force:
                lag_fail = execution("Laguerre", lag_degree, width, fused, None, admitted=False, hit=not cold)
                recs.append(router_record("force_laguerre_first", state,
                                          [plan("Chebyshev", 6, width), plan("Laguerre", lag_degree, width)],
                                          preflight("Laguerre", None), [lag_fail, copy.deepcopy(cheb_ex)]))
            pp_cheb = execution("Chebyshev", 6, width, fused, bound)
            pp_lag = execution("Laguerre", lag_degree, width, fused, None, admitted=False)
            recs.append(pp08_record(state, [attempt(pp_cheb), attempt(pp_lag)]))
        lam, rho_enc, verified = enclosure or (0.0, rho, True)
        timing_rows = None
        if timing:
            timing_rows = []
            for b in pc.BASES:
                rep = report(b, 6, fused, None, execution=pc.EXEC_TIMING)
                rejected = {"admitted": False, "bound": None, "reason": "PP08_ROUTER_NOT_ADMITTED: timing"}
                timing_rows.append({"basis": b, "execution": pc.EXEC_TIMING, "admission_at_total_budget": rejected,
                                    "admission_at_1e300": rejected, "report": rep,
                                    "parity": {"report_bitwise_equal": True, "counters_equal": True,
                                               "replay_fused": list(fused), "replay_candidate_total": None}})
        return {"label": label, "group": group, "family": family, "n": n, "rho_nominal": rho, "seed": seed,
                "input_seed": input_seed, "a": [[hx(x) for x in row] for row in a], "h": hx(h), "input": kind,
                "w": [[hx(x) for x in row] for row in w],
                "enclosure": {"lambda": hx(lam), "rho": hx(rho_enc), "verified": verified},
                "total_budget": hx(budget), "records": recs, "timing": timing_rows}

    def document(self) -> dict:
        grid = pc.grid_cases()
        mats = {}
        cases = []
        for g in grid:
            key = (g["family"], g["n"], g["rho"])
            if key not in mats:
                seed = None if g["family"] == "diagonal" else 7001 + sum(1 for k in mats if k[0] == "rotated")
                a = pc.diagonal_matrix(g["n"], g["rho"]) if seed is None else rotated(g["n"], g["rho"], seed)
                mats[key] = (a, seed)
            a, seed = mats[key]
            enclosure = None if g["family"] == "diagonal" else (0.0, 1.5 * g["rho"], True)
            cases.append(self.case(g["label"], "grid", g["family"], g["n"], g["rho"], seed, g["input_seed"], a,
                                   g["h"], g["input"], g["budget"], enclosure=enclosure,
                                   niche=g["label"] in NICHE_LABELS))
        rot64, s64 = mats[("rotated", 8, 64.0)]
        rot4, s4 = mats[("rotated", 8, 4.0)]
        diag64, _ = mats[("diagonal", 8, 64.0)]
        dmax = max(abs(rot64[i][i]) for i in range(8))
        cases.append(self.case("declared-enclosure", "declared", "rotated", 8, 64.0, s64, 201_000, rot64, 0.05,
                               "distinct", 1e-10, enclosure=(0.0, dmax, False), declared=True))
        cases.append(self.case("unbounded-timing", "timing", "diagonal", 8, 64.0, None, 202_000, diag64, 0.05,
                               "distinct", 1e-10, timing=True))
        cases.append(self.case("laguerre-degree-above-limit", "degree-above-limit", "diagonal", 8, 64.0, None,
                               203_000, diag64, 1.0, "distinct", 1e-10, above=True, force=True))
        cases.append(self.case("total-budget-zero", "budget-zero", "rotated", 8, 4.0, s4, 204_000, rot4, 0.05,
                               "distinct", 0.0, enclosure=(0.0, 6.0, True), zero=True))
        minus3 = [[-3.0 if i == j else 0.0 for j in range(8)] for i in range(8)]
        cases.append(self.case("scalar-minus3I", "scalar", "scalar", 8, 3.0, None, 205_000, minus3, 0.12,
                               "distinct", 1e-10, enclosure=(3.0, 3.0, True), scalar=True))
        # Force: the first grid case of each family (every PP08 Laguerre attempt is rejected).
        selection = []
        for family in pc.GRID_FAMILIES:
            g = next(x for x in grid if x["family"] == family)
            src = next(c for c in cases if c["label"] == g["label"])
            label = "force-" + g["label"]
            a = [[pc.unhex(x) for x in row] for row in src["a"]]
            enc = src["enclosure"]
            case = self.case(label, pc.FORCE_GROUP, family, g["n"], g["rho"], src["seed"], g["input_seed"], a,
                             g["h"], g["input"], g["budget"],
                             enclosure=(pc.unhex(enc["lambda"]), pc.unhex(enc["rho"]), True), force=True)
            cases.append(case)
            selection.append({"family": family, "label": label, "rule": "first-grid-case"})
        return {"schema": pc.CASES_SCHEMA, "node": pc.NODE, "source_commit": COMMIT, "tree_status": "",
                "constants": {"truncation_fraction": 0.25, "degree_limit": 128,
                              "scales": [hx(s) for s in pc.SCALES], "first_rotated_seed": 7001,
                              "lambda_min_fraction": 0.1, "max_givens_angle": 0.3},
                "force_selection": selection, "cases": cases}


_DOC = None


def base_document() -> dict:
    global _DOC
    if _DOC is None:
        with Builder() as b:
            _DOC = b.document()
    return copy.deepcopy(_DOC)


def find(doc: dict, label: str) -> dict:
    return next(c for c in doc["cases"] if c["label"] == label)


def rec(case: dict, arm: str, state: str) -> dict:
    return next(r for r in case["records"] if r["arm"] == arm and r["cache_state"] == state)


class CheckerTests(unittest.TestCase):
    def run_checker(self, doc=None, text=None, *, niche_min=2, blob=CHECKER_BYTES, status="", gz=False):
        with tempfile.TemporaryDirectory() as tmp, Builder(), mock.patch.object(pc, "NICHE_MIN", niche_min):
            cases = Path(tmp) / ("cases.json.gz" if gz else "cases.json")
            data = (text if text is not None else json.dumps(doc)).encode()
            if gz:
                import gzip
                data = gzip.compress(data)
            cases.write_bytes(data)
            out = Path(tmp) / "RESULTS.json"
            code = pc.main(["--cases", str(cases), "--output", str(out)], blob_at=lambda _c: blob,
                           tree_status=lambda: status, head=lambda: COMMIT)
            result = json.loads(out.read_text())
        return code, result

    def assert_invalid(self, doc=None, **kw):
        code, result = self.run_checker(doc, **kw)
        self.assertEqual(code, 2, result.get("reasons", result.get("gate")))
        self.assertEqual(result["verdict"], "INVALID")
        return result

    def assert_fail_on(self, doc, item, **kw):
        code, result = self.run_checker(doc, **kw)
        self.assertEqual(code, 1, result.get("reasons"))
        self.assertEqual(result["verdict"], "FAIL")
        failing = sorted(k for k, v in result["gate"].items() if not v)
        self.assertEqual(failing, [item], result["items"])
        return result

    # ------------------------------------------------------------------ registered constants
    def test_registered_constants(self):
        self.assertEqual(pc.NICHE_MIN, 6)
        self.assertEqual(len(pc.grid_cases()), 96)
        self.assertEqual(len({g["label"] for g in pc.grid_cases()}), 96)
        self.assertEqual(pc.grid_cases()[0]["label"], "diagonal-n8-rho4-h0.005-distinct-b1e-6")
        self.assertEqual(pc.DPS, 50)
        self.assertEqual(pc.DPS_CHECK, 70)
        self.assertEqual(pc.REFERENCE_AGREEMENT, "1e-40")
        self.assertEqual(pc.rust_exp(1e-10), "1e-10")
        self.assertEqual(pc.rust_exp(2.5e-11), "2.5e-11")

    def test_distance_is_evaluated_at_50_digits(self):
        import mpmath
        with mpmath.workdps(50):
            exact = [mpmath.mpf(1) + mpmath.mpf("1e-30")]
        gap = pc.distance([1.0], exact)
        with mpmath.workdps(50):
            self.assertLess(abs(gap - mpmath.mpf("1e-30")), mpmath.mpf("1e-45"))

    # ------------------------------------------------------------------ PASS and niche
    def test_valid_fixture_passes(self):
        code, result = self.run_checker(base_document())
        self.assertEqual(code, 0, result.get("items", result.get("reasons")))
        self.assertEqual(result["verdict"], "PASS")
        self.assertEqual(result["items"]["5_niche_benefit"]["niche_size"], 2)

    def test_gzip_input_is_read(self):
        code, result = self.run_checker(base_document(), gz=True)
        self.assertEqual(code, 0, result.get("reasons"))

    def test_registered_minimum_fails_as_niche_empty(self):
        result = self.assert_fail_on(base_document(), "5_niche_benefit", niche_min=6)
        self.assertTrue(result["items"]["5_niche_benefit"]["problems"][0].startswith("niche empty"))

    def test_no_niche_fails_niche_empty(self):
        doc = base_document()
        for label in NICHE_LABELS:
            for state in pc.STATES:
                r = rec(find(doc, label), "single", state)
                r["preflight"]["preselected"] = "Chebyshev"
        # Not a consistent route any more, but item 5 is decided by the cold preflight alone.
        code, result = self.run_checker(doc)
        self.assertEqual(result["items"]["5_niche_benefit"]["niche_size"], 0)
        self.assertFalse(result["gate"]["5_niche_benefit"])

    # ------------------------------------------------------------------ INVALID
    def test_invalid_strict_json(self):
        text = json.dumps(base_document()).replace('"tree_status": ""', '"tree_status": "", "tree_status": ""', 1)
        self.assert_invalid(text=text)
        self.assert_invalid(text=json.dumps(base_document()).replace("0.25", "NaN", 1))

    def test_invalid_missing_case(self):
        doc = base_document()
        doc["cases"].pop(0)
        self.assert_invalid(doc)

    def test_invalid_duplicate_record(self):
        doc = base_document()
        doc["cases"][0]["records"].append(copy.deepcopy(doc["cases"][0]["records"][0]))
        self.assert_invalid(doc)

    def test_invalid_extra_key(self):
        doc = base_document()
        doc["cases"][1]["records"][0]["extra"] = 1
        self.assert_invalid(doc)

    def test_invalid_hex(self):
        doc = base_document()
        doc["cases"][1]["h"] = doc["cases"][1]["h"].upper()
        self.assert_invalid(doc)

    def test_invalid_bool_as_int(self):
        doc = base_document()
        doc["cases"][1]["records"][0]["cost"]["vector_products"] = True
        self.assert_invalid(doc)

    def test_invalid_diagonal_construction(self):
        doc = base_document()
        case = doc["cases"][0]
        case["a"][1][1] = hx(pc.unhex(case["a"][1][1]) * (1 + 2 ** -52))
        self.assert_invalid(doc)

    def test_invalid_input_stream(self):
        doc = base_document()
        doc["cases"][2]["w"][0][0] = hx(0.5)
        self.assert_invalid(doc)

    def test_invalid_export_arithmetic(self):
        doc = base_document()
        doc["cases"][3]["records"][0]["cost"]["adjoint_sum_terms"] += 1
        self.assert_invalid(doc)

    def test_invalid_dirty_trees(self):
        doc = base_document()
        doc["tree_status"] = " M crates/x.rs\n"
        self.assert_invalid(doc)
        self.assert_invalid(base_document(), status=" M tools/x.py\n")

    def test_invalid_provenance(self):
        self.assert_invalid(base_document(), blob=b"another checker")

    def test_invalid_reference_disagreement(self):
        import mpmath
        real = pc.exact_target

        def shifted(a, h, w, dps):
            out = real(a, h, w, dps)
            return [x * (1 + mpmath.mpf("1e-30")) for x in out] if dps == pc.DPS_CHECK else out

        with mock.patch.object(pc, "exact_target", shifted):
            self.assert_invalid(base_document())

    def test_invalid_force_selection(self):
        doc = base_document()
        doc["force_selection"][0]["rule"] = "fallback-budget"
        self.assert_invalid(doc)

    def test_invalid_timing_outside_the_timing_case(self):
        doc = base_document()
        doc["cases"][0]["timing"] = copy.deepcopy(find(doc, "unbounded-timing")["timing"])
        self.assert_invalid(doc)

    def test_invalid_degree_not_above_limit(self):
        doc = base_document()
        for r in find(doc, "laguerre-degree-above-limit")["records"]:
            if r["arm"] == "single":
                r["plans"][1]["plan"]["degree"] = 100
        self.assert_invalid(doc)

    # ------------------------------------------------------------------ FAIL per item
    def test_fail_certification_error_above_bound(self):
        doc = base_document()
        case = doc["cases"][1]
        bad = hx(pc.unhex(case["records"][0]["fused"][0]) + 1e-3)
        for r in case["records"]:
            if r["arm"] in ("single", "cheb") and r["fused"] is not None:
                r["fused"][0] = bad
                for ex in r["executions"]:
                    ex["report"]["fused"][0] = bad
                    ex["parity"]["replay_fused"][0] = bad
        self.assert_fail_on(doc, "1_certification")

    def test_fail_certification_declared_admitted(self):
        doc = base_document()
        r = rec(find(doc, "declared-enclosure"), "pp08", "cold")
        at = r["attempts"][0]
        at["admission"] = {"admitted": True, "bound": hx(1e-12), "reason": None}
        self.assert_fail_on(doc, "1_certification")

    def test_fail_parity(self):
        doc = base_document()
        doc["cases"][4]["records"][0]["executions"][0]["parity"]["counters_equal"] = False
        self.assert_fail_on(doc, "2_parity")

    def test_fail_parity_single_differs_from_cheb(self):
        doc = base_document()
        case = doc["cases"][1]
        other = hx(pc.unhex(rec(case, "cheb", "cold")["bound"]) * 0.5)
        r = rec(case, "cheb", "cold")
        r["bound"] = other
        r["executions"][0]["admission"]["bound"] = other
        self.assert_fail_on(doc, "2_parity")

    def test_fail_accounting(self):
        doc = base_document()
        doc["cases"][5]["records"][0]["caller_counters"]["poly_block_allocations"] += 1
        self.assert_fail_on(doc, "3_accounting")

    def test_fail_accounting_force_not_charged(self):
        doc = base_document()
        force = next(c for c in doc["cases"] if c["group"] == pc.FORCE_GROUP)
        r = rec(force, "force_laguerre_first", "cold")
        r["executions"][0]["executed"] = False
        r["executions"][0]["cost"]["executed_backends"] = 0
        r["cost"]["executed_backends"] -= 1
        self.assert_fail_on(doc, "3_accounting")

    def test_fail_one_execution(self):
        doc = base_document()
        r = rec(doc["cases"][0], "pp08", "warm")
        for at in r["attempts"]:
            at["vector_products"] = 1
        self.assert_fail_on(doc, "4_one_execution")

    def test_fail_niche_cost(self):
        doc = base_document()
        r = rec(find(doc, NICHE_LABELS[0]), "single", "cold")
        r["preflight"]["cost"]["coefficient_series_terms"] += 1000
        r["cost"]["coefficient_series_terms"] += 1000
        self.assert_fail_on(doc, "5_niche_benefit")


if __name__ == "__main__":
    unittest.main()
