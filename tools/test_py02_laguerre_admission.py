#!/usr/bin/env python3
"""Synthetic tests of tools/py02_laguerre_admission.py (research node PY02, stage 1).

Run: python3 -m unittest discover -s tools -p test_py02_laguerre_admission.py
"""

from __future__ import annotations

import copy
import json
import math
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))
import py02_laguerre_admission as m  # noqa: E402

ROOT = HERE.parent
PY01 = ROOT / "research/py01_laguerre_component_export_20261011/RESULTS.json"
RNEXT04 = ROOT / "research/rnext04_laguerre_admission_20261003/cases.json"
COMMIT = "0123456789abcdef0123456789abcdef01234567"

_WEIGHTS: dict = {}


def up(x: float) -> float:
    return math.nextafter(x, math.inf)


def weight_up(w: list[list[float]]) -> float:
    key = tuple(m.hx(x) for row in w for x in row)
    if key not in _WEIGHTS:
        exact = m.exact_weight_factor(w)
        _WEIGHTS[key] = up(float(exact))
    return _WEIGHTS[key]


def basis(degree: int, width: int, budget: float, scale: float | None) -> dict:
    return {"degree": degree, "laguerre_scale": None if scale is None else m.hx(scale),
            "truncation_bound": m.hx(budget / 2), "vector_products": degree * width, "tail_evaluations": 10,
            "error": None}


def parity(plan: dict) -> dict:
    return {k: plan[k] for k in ("degree", "laguerre_scale", "vector_products", "tail_evaluations", "error")}


def plans_doc(fewer: int = 0, norm_f: float = 0.5) -> dict:
    """The 32 registered cases; the first ``fewer`` have a Laguerre plan with fewer products."""
    cases = []
    for i, spec in enumerate(m.grid_cases()):
        n = spec["n"]
        a = m.generator_matrix(n, spec["rho"], spec["seed"])
        w, v = m.registered_input(spec["input"], n, spec["seed"])
        width = 1 if spec["input"] == "same" else 5
        total = spec["factor"] * max(norm_f, 1.0)
        trunc = total * 0.25
        cheb = basis(30, width, trunc, None)
        lag = basis(20 if i < fewer else 90, width, trunc, 16.0)
        cases.append({
            "label": spec["label"], "n": n, "rho_nominal": m.hx(spec["rho"]), "h": m.hx(spec["h"]),
            "input": spec["input"], "budget_factor": m.hx(spec["factor"]), "seed": spec["seed"],
            "input_seed": spec["input_seed"], "a": [[m.hx(x) for x in row] for row in a],
            "w": [[m.hx(x) for x in row] for row in w],
            "vector": None if v is None else [m.hx(x) for x in v],
            "scales": None if v is None else [m.hx(s) for s in m.INPUT_SCALES],
            "enclosure": {"lambda": m.hx(0.0), "rho": m.hx(1.2 * spec["rho"] * (1.0 + 1.0e-12)), "verified": True},
            "norm_f": m.hx(norm_f), "total_budget": m.hx(total), "truncation_budget": m.hx(trunc),
            "plan": {"branch": "recurrence", "normalization_shift": 0, "width": width,
                     "weight_factor": m.hx(weight_up(w)), "core_budget": m.hx(trunc), "chebyshev": cheb,
                     "laguerre": lag},
            "parity_reported": {"chebyshev": parity(cheb), "laguerre": parity(lag), "equal": True},
        })
    return {"schema": m.PLANS_SCHEMA, "node": m.NODE, "source_commit": COMMIT, "tree_status": "",
            "constants": {"first_seed": 9001, "truncation_fraction": 0.25,
                          "laguerre_scales": [m.hx(s) for s in m.LAGUERRE_SCALES],
                          "budget_factors": [m.hx(f) for f in m.BUDGET_FACTORS],
                          "input_scales": [m.hx(s) for s in m.INPUT_SCALES], "a3_threshold": 8},
            "cases": cases}


def fake_reference(norm: float = 0.5, gap: float = 0.0):
    import mpmath as mp

    def reference(a, h, w):
        return mp.mpf(norm), mp.mpf(gap)
    return reference


class Base(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = plans_doc()
        cls.py01 = json.loads(PY01.read_text())

    def run_checker(self, plans: dict | bytes, py01: dict | None = None, *, reference=None, tree="",
                    blob=None, pin: str | None = m.PY01_SHA256) -> tuple[int, dict]:
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            plans_path = tmp / "PLANS.json"
            plans_path.write_bytes(plans if isinstance(plans, bytes) else json.dumps(plans).encode())
            if py01 is None:
                py01_path = PY01
            else:
                py01_path = tmp / "RESULTS.json"
                py01_path.write_text(json.dumps(py01))
            out = tmp / "ADMISSION.json"
            code = m.main(["--py01", str(py01_path), "--plans", str(plans_path), "--output", str(out)],
                          blob_at=blob or (lambda commit: Path(m.__file__).read_bytes()),
                          tree_status=lambda: tree, head=lambda: COMMIT,
                          reference=reference or fake_reference(), py01_pin=pin)
            return code, json.loads(out.read_text())

    def assert_invalid(self, plans, py01=None, **kw) -> dict:
        code, report = self.run_checker(plans, py01, **kw)
        self.assertEqual(code, 2, report.get("decision"))
        self.assertEqual(report["verdict"], "INVALID")
        return report


class Decisions(Base):
    def test_abstain_a3(self):
        code, r = self.run_checker(self.base)
        self.assertEqual(code, 0)
        self.assertEqual(r["verdict"], "ABSTAIN")
        self.assertEqual(r["decision"], "ABSTAIN (A3)")
        self.assertEqual(r["admission"], {"A1": True, "A2": True, "A3": False})
        self.assertEqual(r["items"]["A3_work_feasibility"]["count"], 0)
        self.assertEqual(r["ledger_mapping"], {"verdict": "PASS", "claim": "decision ABSTAIN (A3)"})
        self.assertTrue(r["reported"]["predictions"]["outcome_ABSTAIN_A3"]["holds"])
        self.assertEqual(r["reported"]["plan_parity_with_py03"]["cases_equal"], 32)
        self.assertIn("not implemented", r["stage2"])

    def test_admit_at_eight_and_not_at_seven(self):
        code, r = self.run_checker(plans_doc(fewer=8))
        self.assertEqual((code, r["decision"], r["ledger_mapping"]), (0, "ADMIT", None))
        self.assertEqual(r["items"]["A3_work_feasibility"]["count"], 8)
        code, r = self.run_checker(plans_doc(fewer=7))
        self.assertEqual((code, r["decision"]), (0, "ABSTAIN (A3)"))

    def test_equal_products_do_not_count(self):
        doc = plans_doc(fewer=0)
        for case in doc["cases"][:8]:
            width = case["plan"]["width"]
            case["plan"]["laguerre"].update(degree=30, vector_products=30 * width)
            case["parity_reported"]["laguerre"] = parity(case["plan"]["laguerre"])
        code, r = self.run_checker(doc)
        self.assertEqual(r["items"]["A3_work_feasibility"]["count"], 0)

    def test_failed_plan_does_not_count(self):
        doc = plans_doc(fewer=8)
        lag = doc["cases"][0]["plan"]["laguerre"]
        for k in m.BASIS_KEYS:
            lag[k] = None
        lag["error"] = "POLYNOMIAL_DOMAIN_OR_ACCURACY_UNSUPPORTED: Laguerre degree above 4096"
        doc["cases"][0]["parity_reported"]["laguerre"] = parity(lag)
        code, r = self.run_checker(doc)
        self.assertEqual((code, r["decision"]), (0, "ABSTAIN (A3)"))
        self.assertEqual(r["items"]["A3_work_feasibility"]["count"], 7)

    def test_a1_fails_first(self):
        py01 = copy.deepcopy(self.py01)
        case = py01["reported"]["cases"]["n12-rho400-h0.1-same"]
        case["terms_exact"]["recurrence_adjoint"] = str(0.89 * float(case["S"]))
        code, r = self.run_checker(self.base, py01, pin=None)
        self.assertEqual((code, r["decision"], r["first_failed"]), (0, "ABSTAIN (A1)", "A1"))
        self.assertEqual(r["ledger_mapping"]["claim"], "decision ABSTAIN (A1)")
        self.assertFalse(r["items"]["A1_dominance"]["cases"]["n12-rho400-h0.1-same"]["holds"])

    def test_a1_threshold_is_inclusive_with_rounding_margin(self):
        py01 = copy.deepcopy(self.py01)
        case = py01["reported"]["cases"]["n6-rho400-h0.1-same"]
        case["S"] = "1"
        case["terms_exact"]["recurrence_adjoint"] = "0.9"
        code, r = self.run_checker(self.base, py01, pin=None)
        # The lower bound 0.9 (1 - 1e-11) / (1 + 1e-11) is below 0.9: conservative.
        self.assertFalse(r["admission"]["A1"])
        case["terms_exact"]["recurrence_adjoint"] = "0.90000000002"
        code, r = self.run_checker(self.base, py01, pin=None)
        self.assertTrue(r["admission"]["A1"])

    def test_a2_needs_one_case(self):
        py01 = copy.deepcopy(self.py01)
        for label in m.GATED:
            py01["reported"]["cases"][label]["closability_floor_F"] = "2e-6"
        code, r = self.run_checker(self.base, py01, pin=None)
        self.assertEqual(r["decision"], "ABSTAIN (A2)")
        py01["reported"]["cases"]["n20-rho400-h0.1-same"]["closability_floor_F"] = "1e-6"
        code, r = self.run_checker(self.base, py01, pin=None)
        self.assertFalse(r["admission"]["A2"])  # F (1 + 1e-11) > budget: conservative
        py01["reported"]["cases"]["n20-rho400-h0.1-same"]["closability_floor_F"] = "9.9e-7"
        code, r = self.run_checker(self.base, py01, pin=None)
        self.assertTrue(r["admission"]["A2"])
        self.assertEqual(r["items"]["A2_closability"]["cases_holding"], 1)

    def test_no_component_evidence(self):
        for mutate in (lambda d: d.update(verdict="FAIL"), lambda d: d.update(verdict="INVALID"),
                       lambda d: d["gate"].update({"1_completeness": False})):
            py01 = copy.deepcopy(self.py01)
            mutate(py01)
            code, r = self.run_checker(plans_doc(fewer=8), py01, pin=None)
            self.assertEqual((code, r["decision"]), (0, "ABSTAIN (no component evidence)"))
            self.assertEqual(r["ledger_mapping"]["verdict"], "INCONCLUSIVE")

    def test_real_py01_items(self):
        code, r = self.run_checker(self.base)
        a1, a2 = r["items"]["A1_dominance"], r["items"]["A2_closability"]
        self.assertEqual(len(a1["cases"]), 6)
        self.assertEqual(a2["cases_holding"], 6)
        self.assertTrue(all(c["recorded_F_at_most_budget"] for c in a2["cases"].values()))

    def test_output_is_immutable(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "ADMISSION.json"
            out.write_text("{}")
            with self.assertRaises(SystemExit):
                m.main(["--py01", str(PY01), "--plans", str(out), "--output", str(out)])


class Invalid(Base):
    def mutated(self, fn) -> dict:
        doc = copy.deepcopy(self.base)
        fn(doc)
        return doc

    def test_pin(self):
        py01 = copy.deepcopy(self.py01)
        self.assert_invalid(self.base, py01)

    def test_strict_json(self):
        text = json.dumps(self.base)
        self.assert_invalid(text.replace('"tree_status": ""', '"tree_status": NaN', 1).encode())
        self.assert_invalid(text.replace('"tree_status": ""', '"tree_status": "", "tree_status": ""', 1).encode())

    def test_keys_and_types(self):
        self.assert_invalid(self.mutated(lambda d: d.update(extra=1)))
        self.assert_invalid(self.mutated(lambda d: d["cases"][3].pop("norm_f")))
        self.assert_invalid(self.mutated(lambda d: d["cases"][3]["plan"]["chebyshev"].update(degree=True)))
        self.assert_invalid(self.mutated(lambda d: d["cases"][3]["plan"]["chebyshev"].update(degree=30.0)))
        self.assert_invalid(self.mutated(lambda d: d["cases"][3].update(norm_f="3FE0000000000000")))
        self.assert_invalid(self.mutated(lambda d: d["cases"][3]["parity_reported"].update(equal=1)))

    def test_case_set(self):
        self.assert_invalid(self.mutated(lambda d: d["cases"].pop()))
        self.assert_invalid(self.mutated(lambda d: d["cases"].reverse()))
        self.assert_invalid(self.mutated(lambda d: d["cases"].append(copy.deepcopy(d["cases"][0]))))
        self.assert_invalid(self.mutated(lambda d: d["cases"][0].update(label="n6-rho400-h0.1-distinct-b1e-6")))

    def test_binding(self):
        def flip_a(d):
            d["cases"][5]["a"][1][0] = m.hx(m.unhex(d["cases"][5]["a"][1][0]) * 1.5)
        self.assert_invalid(self.mutated(flip_a))

        def flip_w(d):
            d["cases"][6]["w"][2][1] = m.hx(0.125)
        self.assert_invalid(self.mutated(flip_w))
        self.assert_invalid(self.mutated(lambda d: d["cases"][0].update(seed=9000)))
        self.assert_invalid(self.mutated(lambda d: d["cases"][0].update(input_seed=9001)))
        self.assert_invalid(self.mutated(lambda d: d["cases"][2]["enclosure"].update(verified=False)))
        self.assert_invalid(self.mutated(lambda d: d["cases"][2]["enclosure"].update(rho=m.hx(120.0))))
        self.assert_invalid(self.mutated(lambda d: d["constants"].update(first_seed=9002)))
        self.assert_invalid(self.mutated(lambda d: d["constants"].update(a3_threshold=7)))

    def test_budgets(self):
        self.assert_invalid(self.mutated(lambda d: d["cases"][1].update(total_budget=m.hx(2e-6))))
        self.assert_invalid(self.mutated(lambda d: d["cases"][1].update(truncation_budget=m.hx(1e-6))))
        # norm_f against the 50-digit reference.
        self.assert_invalid(plans_doc(norm_f=1.5), reference=fake_reference(1.6))
        code, _ = self.run_checker(plans_doc(norm_f=1.5), reference=fake_reference(1.5 * (1 + 1e-13)))
        self.assertEqual(code, 0)
        # Below one both budgets use 1: agreement only on max(||F||, 1).
        code, _ = self.run_checker(self.base, reference=fake_reference(0.25))
        self.assertEqual(code, 0)
        self.assert_invalid(self.base, reference=fake_reference(0.5, gap=1e-30))

    def test_plans(self):
        def products(d):
            d["cases"][4]["plan"]["laguerre"]["vector_products"] += 1
        self.assert_invalid(self.mutated(products))
        self.assert_invalid(self.mutated(
            lambda d: d["cases"][4]["plan"]["chebyshev"].update(truncation_bound=m.hx(1.0))))
        self.assert_invalid(self.mutated(
            lambda d: d["cases"][4]["plan"]["chebyshev"].update(laguerre_scale=m.hx(16.0))))
        self.assert_invalid(self.mutated(
            lambda d: d["cases"][4]["plan"]["laguerre"].update(laguerre_scale=m.hx(3.0))))
        self.assert_invalid(self.mutated(
            lambda d: d["cases"][4]["plan"]["laguerre"].update(error="also failed")))
        self.assert_invalid(self.mutated(
            lambda d: d["cases"][4]["plan"]["laguerre"].update(tail_evaluations=0)))
        self.assert_invalid(self.mutated(lambda d: d["cases"][4]["plan"].update(branch="scalar")))
        self.assert_invalid(self.mutated(lambda d: d["cases"][4]["plan"].update(normalization_shift=3)))
        self.assert_invalid(self.mutated(lambda d: d["cases"][4]["plan"].update(width=5 if d["cases"][4]["input"]
                                                                                 == "same" else 1)))
        self.assert_invalid(self.mutated(lambda d: d["cases"][4]["plan"].update(core_budget=m.hx(1e-9))))

        def weight_low(d):
            p = d["cases"][4]["plan"]
            p["weight_factor"] = m.hx(m.unhex(p["weight_factor"]) * (1 - 1e-9))
        self.assert_invalid(self.mutated(weight_low))

        def weight_high(d):
            p = d["cases"][4]["plan"]
            p["weight_factor"] = m.hx(m.unhex(p["weight_factor"]) * (1 + 1e-9))
        self.assert_invalid(self.mutated(weight_high))

    def test_provenance(self):
        self.assert_invalid(self.mutated(lambda d: d.update(tree_status=" M tools/x.py\n")))
        self.assert_invalid(self.base, tree=" M crates/rodas5p-core/src/polynomial_action.rs\n")
        self.assert_invalid(self.base, blob=lambda commit: b"another checker")
        self.assert_invalid(self.mutated(lambda d: d.update(source_commit="abc")))
        report = self.assert_invalid(self.mutated(lambda d: d.update(schema="other")))
        self.assertIsNone(report["gate"])


class Helpers(unittest.TestCase):
    def test_grid(self):
        grid = m.grid_cases()
        self.assertEqual(len(grid), 32)
        self.assertEqual(len({g["label"] for g in grid}), 32)
        self.assertEqual(grid[0]["label"], "n7-rho100-h0.08-distinct-b1e-6")
        self.assertEqual(grid[-1]["label"], "n14-rho300-h0.16-same-b1e-8")
        self.assertEqual(sorted({g["seed"] for g in grid}), list(range(9001, 9009)))
        self.assertEqual(m.rust_exp(1e-8), "1e-8")
        self.assertEqual(m.rust_display(0.16), "0.16")

    def test_generator_is_rnext04(self):
        doc = json.loads(RNEXT04.read_text())
        first, second = doc["cases"][0], doc["cases"][1]
        self.assertEqual(first["label"], "n6-rho1-h0.001-distinct")
        a = m.generator_matrix(6, 1.0, 2)
        self.assertEqual(first["a"], [[m.hx(x) for x in row] for row in a])
        # R-NEXT-04's operator seed 2: distinct from 2 + 9, same vector from 2 + 5.
        w, _ = m.registered_input("distinct", 6, 2)
        self.assertEqual(first["w"], [[m.hx(x) for x in row] for row in w])
        w, _ = m.registered_input("same", 6, 2)
        self.assertEqual(second["w"], [[m.hx(x) for x in row] for row in w])

    def test_reference_scalar(self):
        import mpmath as mp
        # A = [-2], h = 0.5: F = e^-1 w0 + phi_1(-1) w1.
        f = m.exact_target([[-2.0]], 0.5, [[1.0], [1.0], [0.0], [0.0], [0.0]], 50)
        with mp.workdps(50):
            expected = mp.e ** -1 + (1 - mp.e ** -1)
            self.assertLess(abs(f[0] - expected), mp.mpf(10) ** -45)
        norm, gap = m.reference_norm([[-2.0, 0.0], [0.0, -1.0]], 0.5,
                                     [[1.0, 0.0], [0.0, 0.0], [0.0, 0.0], [0.0, 0.0], [0.0, 1.0]])
        self.assertLess(gap, mp.mpf(10) ** -45)

    def test_emulated_degrees(self):
        e1 = m.emulated_degrees(0.1, 120.0, 3.0, 1e-8)
        e2 = m.emulated_degrees(0.1, 120.0, 3.0, 1e-12)
        self.assertLessEqual(e1["chebyshev"], e2["chebyshev"])
        self.assertLessEqual(e1["laguerre"], e2["laguerre"])
        self.assertIn(e1["laguerre_scale"], m.LAGUERRE_SCALES)

    def test_disjoint_grids(self):
        for g in m.grid_cases():
            for ns, rhos, hs in m.DISJOINT.values():
                self.assertFalse(g["n"] in ns and g["rho"] in rhos and g["h"] in hs)


if __name__ == "__main__":
    unittest.main()
