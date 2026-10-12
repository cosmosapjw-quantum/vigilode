#!/usr/bin/env python3
"""Synthetic tests of tools/py04_fused_normalization_check.py (research node PY04).

The fixture is a complete, registered-shape cases document built here from the checker's own regeneration of the 144
inputs, with fabricated certificates that are consistent with a cheap linear stand-in reference F(w) = sum_k w_k (so
no expm runs). It exercises PASS, every INVALID class and a FAIL of each gate item. The real mpmath reference is
checked separately on closed forms. Run: python3 -m unittest discover -s tools -p test_py04_fused_normalization_check.py
"""

from __future__ import annotations

import copy
import json
import math
import sys
import tempfile
import unittest
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import py04_fused_normalization_check as chk  # noqa: E402

import mpmath as mp  # noqa: E402

CHECKER = Path(chk.__file__).resolve()
TINY = Fraction(1, 2 ** 1074)


def up(x: Fraction) -> float:
    y = chk.nearest(x)
    if math.isfinite(y) and Fraction(y) < x:
        y = math.nextafter(y, math.inf)
    return y


def linear_reference(h, a, w, dps):
    with mp.workdps(dps):
        return [mp.fsum(chk.mpf_exact(Fraction(w[k][i])) for k in range(5)) for i in range(len(a))]


def skewed_reference(h, a, w, dps):
    """Disagrees between 50 and 70 digits by far more than the tolerance."""
    out = linear_reference(h, a, w, dps)
    return out if dps == chk.DPS else [x * (1 + mp.mpf("1e-20")) for x in out]


def inner(n: int, big_e: float) -> dict:
    return {"schema": chk.INNER_SCHEMA, "status": "bounded", "unbounded_reason": None, "bound": chk.hx(big_e),
            "candidate_distance": chk.hx(big_e / 4), "stepped_error": chk.hx(big_e / 4),
            "perturbation": chk.hx(big_e / 2), "dominant": "perturbation", "delta_two_norm": chk.hx(1e-17),
            "omega": chk.hx(3.0), "v_two_norm": chk.hx(2.0), "chosen_metric": "identity",
            "stepped_candidate": [chk.hx(0.5)] * (n + 4),
            "attempts": [{"metric": "identity", "steps": 2, "bounded": True, "error_upper": chk.hx(big_e / 4)},
                         {"metric": "osborne", "steps": 2, "bounded": True, "error_upper": chk.hx(big_e / 2)}]}


def build_case(reg: dict) -> dict:
    n = reg["n"]
    w = [[Fraction(x) for x in wk] for wk in reg["w"]]
    largest = max(abs(x) for wk in w for x in wk)
    e = chk.floor_log2(largest)
    sigma = Fraction(2) ** e
    v = [[chk.nearest(x / sigma) for x in wk] for wk in w]
    inexact_v = [sum(Fraction(v[k][i]) != w[k][i] / sigma for i in range(n)) for k in range(5)]
    f_true = [sum(w[k][i] for k in range(5)) for i in range(n)]
    z = [chk.nearest(x / sigma) for x in f_true]
    if reg["family"] == "S3":
        # One ulp off in normalized units, so sigma z needs bits below 2^-1074 (R > 0).
        z = [math.nextafter(x, math.inf) for x in z]
    fhat = [chk.nearest(Fraction(zi) * sigma) for zi in z]
    taylor_ok = reg["family"] in ("S1", "S7")
    case = {k: reg[k] for k in ("label", "family", "variant", "w_exponents", "matrix_index", "seed", "symmetric",
                                 "n", "target_hA_one_norm", "h_power_of_two")}
    case.update({"h": chk.hx(reg["h"]), "a": [[chk.hx(x) for x in row] for row in reg["a"]],
                 "base_w": [[chk.hx(x) for x in wk] for wk in reg["base_w"]],
                 "w": [[chk.hx(x) for x in wk] for wk in reg["w"]]})
    if any(not math.isfinite(x) for x in fhat):
        taylor_ok = False
        normalized = {"outcome": "refused", "certificate": None,
                      "error": "invalid numerical input: FUSED_PHI_RANGE_UNSUPPORTED: fused phi normalized "
                               "certificate: a rescaled candidate entry is above f64::MAX"}
    else:
        r_inexact = sum(Fraction(fh) != Fraction(zi) * sigma for fh, zi in zip(fhat, z))
        r = float((r_inexact + 1) * TINY) if r_inexact else 0.0
        big_e = 2.0 ** -40
        scaled = up(Fraction(big_e) * sigma)
        ell = float(Fraction(2) ** (e - 1060)) if sum(inexact_v) else 0.0
        bound = up(Fraction(scaled) + Fraction(r) + Fraction(ell))
        cert = {"schema": chk.NORMALIZED_SCHEMA, "status": "bounded", "unbounded_reason": None, "exponent": e,
                "normalized_vectors": [[chk.hx(x) for x in vk] for vk in v], "normalization_inexact": inexact_v,
                "normalized_candidate": [chk.hx(x) for x in z], "normalized_source": "taylor",
                "normalized_taylor_rejection": None, "inner": inner(n, big_e),
                "candidate": [chk.hx(x) for x in fhat], "rescale_inexact": r_inexact,
                "scaled_certificate": chk.hx(scaled), "rescale_rounding": chk.hx(r),
                "normalization_rounding": chk.hx(ell), "log_norm_upper": chk.hx(-0.5), "bound": chk.hx(bound),
                "dominant": "scaled-certificate",
                "caller_candidate": [chk.hx(x) for x in fhat] if taylor_ok else None,
                "caller_distance": chk.hx(0.0) if taylor_ok else None,
                "caller_bound": chk.hx(bound) if taylor_ok else None,
                "work": {"input_entries": 5 * n, "input_stages": 5 * n, "candidate_entries": n,
                         "candidate_stages": n, "bound_scalings": 2, "bound_stages": 2,
                         "entries_scaled": 6 * n + 2, "stages": 6 * n + 2}}
        normalized = {"outcome": "certificate", "error": None, "certificate": cert}
    case["taylor_original"] = ({"accepted": True, "fused": [chk.hx(x) for x in fhat], "rejection": None}
                               if taylor_ok else {"accepted": False, "fused": None, "rejection": "rejected"})
    case["normalized"] = normalized
    unb = inner(n, 1.0)
    unb.update({"status": "unbounded", "bound": None, "unbounded_reason": "the perturbation term overflows"})
    case["baseline"] = {"outcome": "certificate", "error": None, "candidate_source": "zeros",
                        "candidate": [chk.hx(0.0)] * n, "certificate": unb}
    return case


def build_table() -> list[dict]:
    rows = []
    for x, e in chk.registered_table_inputs():
        exact = Fraction(x) * Fraction(2) ** e
        s = {"x": chk.hx(x), "e": e, "stages": 2, "largest_stage": 1000}
        if abs(exact) >= Fraction(2) ** 1024:
            s.update({"nearest": None, "up": None, "down": None, "exact": False, "overflow": True})
        else:
            near = chk.nearest(exact)
            hi = near if Fraction(near) >= exact else math.nextafter(near, math.inf)
            lo = near if Fraction(near) <= exact else math.nextafter(near, -math.inf)
            s.update({"nearest": chk.hx(near), "up": chk.hx(hi), "down": chk.hx(lo),
                      "exact": Fraction(near) == exact, "overflow": False})
        rows.append(s)
    return rows


FIXTURE = None


def fixture() -> dict:
    global FIXTURE
    if FIXTURE is None:
        FIXTURE = {"schema": chk.CASES_SCHEMA, "node": chk.NODE, "source_commit": "a" * 40, "tree_status": "",
                   "taylor_budget": chk.hx(chk.TAYLOR_BUDGET), "normalized_taylor_budget": chk.hx(chk.TAYLOR_BUDGET),
                   "seed_base": chk.SEED_BASE, "table_seed": chk.TABLE_SEED,
                   "cases": [build_case(r) for r in chk.registered_cases()], "scaling_table": build_table()}
    return copy.deepcopy(FIXTURE)


def find(doc: dict, variant: str, n: int = 5, index: int = 0) -> dict:
    return [c for c in doc["cases"] if c["variant"] == variant and c["n"] == n][index]


class Run:
    def __init__(self, doc=None, text=None, reference=linear_reference, blob=None, status=""):
        self.dir = tempfile.TemporaryDirectory()
        self.cases = Path(self.dir.name) / "cases.json"
        self.output = Path(self.dir.name) / "RESULTS.json"
        self.cases.write_text(text if text is not None else json.dumps(doc))
        self.code = chk.main(["--cases", str(self.cases), "--output", str(self.output)],
                             blob_at=lambda commit: blob if blob is not None else CHECKER.read_bytes(),
                             tree_status=lambda: status, head=lambda: "b" * 40, reference=reference)
        self.report = json.loads(self.output.read_text())
        self.dir.cleanup()


class CheckerTests(unittest.TestCase):
    def test_pass_fixture(self):
        run = Run(fixture())
        self.assertEqual(run.code, 0, run.report.get("gate_failures"))
        self.assertEqual(run.report["verdict"], "PASS")
        self.assertTrue(all(run.report["gate"].values()))
        self.assertEqual(run.report["reported"]["triples_all_exact"], 16)
        s5 = [r["S5_outcome"] for r in run.report["rows"] if r["family"] == "S5"]
        self.assertIn("refused", s5)
        s6 = [r for r in run.report["rows"] if r["family"] == "S6"]
        self.assertTrue(all(r["accounting"]["L"]["v_exact"] is False for r in s6))

    # INVALID ------------------------------------------------------------------------------------------
    def assertInvalid(self, run, fragment):
        self.assertEqual(run.code, 2)
        self.assertEqual(run.report["verdict"], "INVALID")
        self.assertTrue(any(fragment in r for r in run.report["reasons"]), run.report["reasons"][:5])

    def test_duplicate_key_is_invalid(self):
        text = json.dumps(fixture())
        text = text.replace('"schema": "vigilode-py04-cases-v1"',
                            '"schema": "vigilode-py04-cases-v1", "schema": "vigilode-py04-cases-v1"', 1)
        self.assertInvalid(Run(text=text), "duplicate object key")

    def test_nan_constant_is_invalid(self):
        text = json.dumps(fixture()).replace('"seed_base": 41000', '"seed_base": NaN', 1)
        self.assertInvalid(Run(text=text), "non-standard JSON constant")

    def test_missing_case_is_invalid(self):
        doc = fixture()
        doc["cases"].pop()
        self.assertInvalid(Run(doc), "case set")

    def test_duplicate_case_is_invalid(self):
        doc = fixture()
        doc["cases"][-1] = copy.deepcopy(doc["cases"][0])
        self.assertInvalid(Run(doc), "duplicate labels")

    def test_extra_key_is_invalid(self):
        doc = fixture()
        doc["cases"][3]["normalized"]["certificate"]["extra"] = 1
        self.assertInvalid(Run(doc), "key set differs")

    def test_noncanonical_hex_is_invalid(self):
        doc = fixture()
        cert = doc["cases"][0]["normalized"]["certificate"]
        cert["bound"] = "0X" + cert["bound"][2:]
        self.assertInvalid(Run(doc), "16-digit lowercase")

    def test_unbound_input_is_invalid(self):
        doc = fixture()
        case = find(doc, "S3")
        bits = int(case["w"][2][1], 16) ^ 1
        case["w"][2][1] = f"{bits:016x}"
        self.assertInvalid(Run(doc), "regenerated registered input")

    def test_wrong_family_rule_is_invalid(self):
        doc = fixture()
        case = find(doc, "S6")
        case["w_exponents"] = [1000, 0, 0, 0, -1068]
        self.assertInvalid(Run(doc), "w_exponents")

    def test_caller_candidate_must_be_the_taylor_output(self):
        doc = fixture()
        cert = find(doc, "S1")["normalized"]["certificate"]
        cert["caller_candidate"][0] = chk.hx(0.0)
        self.assertInvalid(Run(doc), "Taylor output")

    def test_table_input_is_bound(self):
        doc = fixture()
        doc["scaling_table"][5]["e"] = 7
        self.assertInvalid(Run(doc), "registered seeded table input")

    def test_dirty_export_tree_is_invalid(self):
        doc = fixture()
        doc["tree_status"] = " M crates/rodas5p-core/src/directed.rs\n"
        self.assertInvalid(Run(doc), "export tree was dirty")

    def test_dirty_checker_tree_is_invalid(self):
        self.assertInvalid(Run(fixture(), status=" M tools/x.py\n"), "checker tree is dirty")

    def test_checker_blob_mismatch_is_invalid(self):
        self.assertInvalid(Run(fixture(), blob=b"other"), "differs from the checker run")

    def test_reference_disagreement_is_invalid(self):
        self.assertInvalid(Run(fixture(), reference=skewed_reference), "references differ")

    # FAIL ---------------------------------------------------------------------------------------------
    def assertFails(self, run, item):
        self.assertEqual(run.code, 1, run.report.get("reasons"))
        self.assertEqual(run.report["verdict"], "FAIL")
        self.assertFalse(run.report["gate"][item])
        others = {k: v for k, v in run.report["gate"].items() if k != item}
        return others

    def test_enclosure_violation_fails_item_1(self):
        doc = fixture()
        cert = find(doc, "S4")["normalized"]["certificate"]
        f_hat = [chk.f(x) for x in cert["candidate"]]
        cert["candidate"][0] = chk.hx(f_hat[0] * (1 + 2 ** -20))
        self.assertFails(Run(doc), "1_enclosure")

    def test_caller_violation_fails_item_1(self):
        doc = fixture()
        case = find(doc, "S7-s0")
        u = [chk.f(x) for x in case["taylor_original"]["fused"]]
        u[0] = u[0] + 1e-3
        case["taylor_original"]["fused"] = [chk.hx(x) for x in u]
        case["normalized"]["certificate"]["caller_candidate"] = [chk.hx(x) for x in u]
        self.assertFails(Run(doc), "1_enclosure")

    def test_inner_difference_fails_item_2(self):
        doc = fixture()
        case = find(doc, "S7-p600")
        case["normalized"]["certificate"]["inner"]["perturbation"] = chk.hx(2.0 ** -42)
        others = self.assertFails(Run(doc), "2_homogeneity")
        self.assertTrue(all(others.values()))

    def test_unscaled_bound_fails_item_2(self):
        doc = fixture()
        cert = find(doc, "S7-m600")["normalized"]["certificate"]
        cert["bound"] = chk.hx(math.nextafter(chk.f(cert["bound"]), math.inf))
        self.assertFails(Run(doc), "2_homogeneity")

    def test_nonzero_l_for_exact_v_fails_item_3(self):
        doc = fixture()
        cert = find(doc, "S2")["normalized"]["certificate"]
        cert["normalization_rounding"] = chk.hx(1e-320)
        cert["bound"] = chk.hx(chk.f(cert["bound"]) + 1e-320)
        self.assertFails(Run(doc), "3_normalization_accounting")

    def test_zero_r_for_inexact_rescale_fails_item_3(self):
        doc = fixture()
        cert = next(c["normalized"]["certificate"] for c in doc["cases"]
                    if c["normalized"]["outcome"] == "certificate"
                    and c["normalized"]["certificate"]["rescale_inexact"] > 0)
        cert["rescale_rounding"] = chk.hx(0.0)
        self.assertFails(Run(doc), "3_normalization_accounting")

    def test_unenclosed_l_fails_item_3(self):
        doc = fixture()
        cert = find(doc, "S6")["normalized"]["certificate"]
        cert["normalization_rounding"] = chk.hx(5e-324)
        self.assertFails(Run(doc), "3_normalization_accounting")

    def test_null_bound_in_a_bounded_certificate_fails(self):
        doc = fixture()
        find(doc, "S4")["normalized"]["certificate"]["bound"] = None
        run = Run(doc)
        self.assertFails(run, "3_normalization_accounting")
        self.assertFalse(run.report["gate"]["1_enclosure"])

    def test_wrong_table_row_fails_item_4(self):
        doc = fixture()
        row = next(r for r in doc["scaling_table"] if r["e"] == -1074 and r["exact"] is False)
        row["up"] = row["down"]
        self.assertFails(Run(doc), "4_directed_scaling")

    def test_wrong_exact_flag_fails_item_4(self):
        doc = fixture()
        row = next(r for r in doc["scaling_table"] if r["e"] == 0)
        row["exact"] = False
        self.assertFails(Run(doc), "4_directed_scaling")

    def test_useless_s3_bound_fails_item_5(self):
        doc = fixture()
        cert = find(doc, "S3")["normalized"]["certificate"]
        cert["scaled_certificate"] = chk.hx(1e-300)
        cert["bound"] = chk.hx(1e-300)
        others = self.assertFails(Run(doc), "5_usefulness_new_extremes")
        self.assertTrue(all(others.values()))

    def test_unbounded_s1_fails_item_5(self):
        doc = fixture()
        cert = find(doc, "S1", 12)["normalized"]["certificate"]
        cert["status"] = "unbounded"
        cert["bound"] = None
        self.assertFails(Run(doc), "5_usefulness_new_extremes")

    def test_s5_error_fails_item_6(self):
        doc = fixture()
        case = next(c for c in doc["cases"] if c["family"] == "S5" and c["normalized"]["outcome"] == "refused")
        case["normalized"]["outcome"] = "error"
        case["normalized"]["error"] = "invalid numerical input: something else"
        self.assertFails(Run(doc), "6_overflow_boundary")


class ArithmeticTests(unittest.TestCase):
    def test_nearest_ties_and_overflow(self):
        self.assertEqual(chk.nearest(Fraction(3, 2) * TINY), 2 * 5e-324)
        self.assertEqual(chk.nearest(Fraction(5, 4) * TINY), 5e-324)
        self.assertEqual(chk.nearest(TINY / 2), 0.0)
        self.assertEqual(chk.nearest(Fraction(2) ** 1024), math.inf)
        self.assertEqual(chk.nearest(-Fraction(2) ** 1024), -math.inf)

    def test_floor_log2(self):
        self.assertEqual(chk.floor_log2(TINY), -1074)
        self.assertEqual(chk.floor_log2(Fraction(3) * TINY), -1073)
        self.assertEqual(chk.floor_log2(Fraction(sys.float_info.max)), 1023)
        self.assertEqual(chk.floor_log2(Fraction(3, 4)), -1)
        self.assertEqual(chk.floor_log2(Fraction(1)), 0)

    def test_table_row_check(self):
        good = {"x": chk.hx(1.5), "e": -1074, "nearest": chk.hx(1e-323), "up": chk.hx(1e-323),
                "down": chk.hx(5e-324), "exact": False, "overflow": False, "stages": 3, "largest_stage": 1000}
        self.assertTrue(chk.table_row_check(good)["holds"])
        bad = dict(good, nearest=chk.hx(5e-324))
        self.assertFalse(chk.table_row_check(bad)["holds"])
        over = {"x": chk.hx(2.0), "e": 1023, "nearest": None, "up": None, "down": None, "exact": False,
                "overflow": True, "stages": 2, "largest_stage": 1000}
        self.assertTrue(chk.table_row_check(over)["holds"])
        self.assertFalse(chk.table_row_check(dict(over, nearest=chk.hx(1.0)))["holds"])

    def test_reference_closed_forms(self):
        # A = -1, h = 0.5, w_0 = 2^-1060: F = e^-1/2 2^-1060.
        w = [[Fraction(1, 2 ** 1060)], [Fraction(0)], [Fraction(0)], [Fraction(0)], [Fraction(0)]]
        got = chk.fused_reference(0.5, [[-1.0]], w, 50)[0]
        with mp.workdps(50):
            self.assertLess(abs(got / mp.ldexp(mp.exp(mp.mpf(-0.5)), -1060) - 1), mp.mpf("1e-45"))
        # A = 0: F = w_0 + w_1 + w_2 / 2 + w_3 / 6 + w_4 / 24, at 2^1023 scale.
        big = Fraction(2) ** 1023
        w = [[big], [big], [big], [big], [big]]
        got = chk.fused_reference(0.25, [[0.0]], w, 50)[0]
        with mp.workdps(50):
            exact = mp.ldexp(1 + 1 + mp.mpf(1) / 2 + mp.mpf(1) / 6 + mp.mpf(1) / 24, 1023)
            self.assertLess(abs(got / exact - 1), mp.mpf("1e-45"))
        # phi_1(z) = (e^z - 1) / z at z = h a = -0.3 0.5 (rounded product handled exactly).
        w = [[Fraction(0)], [Fraction(1)], [Fraction(0)], [Fraction(0)], [Fraction(0)]]
        got = chk.fused_reference(0.3, [[-0.5]], w, 50)[0]
        with mp.workdps(50):
            zz = mp.mpf(0.3) * mp.mpf(-0.5)
            self.assertLess(abs(got - mp.expm1(zz) / zz), mp.mpf("1e-45"))


if __name__ == "__main__":
    unittest.main()
