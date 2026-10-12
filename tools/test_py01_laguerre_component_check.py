#!/usr/bin/env python3
"""Synthetic tests for tools/py01_laguerre_component_check.py (research node PY01).

Every fixture is synthetic: a published cases file, a PP09 results file and a COMPONENTS.json with the nine
registered labels, small random operators (n = 2) and degree 3, built consistently with the registered construction
and written to a temporary directory. No Rust code runs and no committed research file is read or written. The
input pins, the git provenance hooks, the sample count and the worker count are patched in-process:
- a valid fixture PASSes (exit 0);
- each validity condition violated on its own -> INVALID (exit 2);
- each gate item violated on its own -> FAIL (exit 1).

Run: python3 -m unittest discover -s tools -p test_py01_laguerre_component_check.py
"""

from __future__ import annotations

import copy
import hashlib
import json
import math
import random
import struct
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import py01_laguerre_component_check as pc  # noqa: E402

M = 3
N = 2
SHIFTS = {"subnormal-1e-310": -1029, "large-1e300": 997}


def hx(x: float) -> str:
    return struct.pack(">d", x).hex()


def build(seed: int = 7) -> tuple[dict, dict, dict]:
    """(published, pp09, components) for the nine labels, consistent with every gate item."""
    rng = random.Random(seed)
    published_cases, components_cases, pp09_rows = [], [], []
    for index, label in enumerate(pc.CASES):
        rho = pc.registered_rho(label)
        rho_enc = 1.2 * rho * (1.0 + 1.0e-12)
        d1, d2 = rng.uniform(0.4, 0.8) * rho, rng.uniform(0.4, 0.8) * rho
        off = rng.uniform(-0.05, 0.05) * rho
        a = [[-d1, off], [off, -d2]]
        h = 0.1 + 0.001 * index
        same = pc.registered_input(label) == "same-vector"
        v = [rng.uniform(-1, 1) for _ in range(N)]
        w = ([[s * x for x in v] for s in pc.SCALES] if same
             else [[rng.uniform(-1, 1) for _ in range(N)] for _ in range(5)])
        scales = [abs(s) for s in pc.SCALES] if same else [1.0] * 5
        shift = SHIFTS.get(label, 0)
        extent = 16.0
        beta = rho_enc / 16.0
        adjoint, terms, digests = [], [], []
        for k in range(5):
            coefficients = [rng.uniform(-1, 1) * 0.5 ** n for n in range(M + 1)]
            envelopes = [rng.uniform(1, 100) for _ in range(M + 1)]
            residuals = [rng.uniform(1e-17, 1e-15) for _ in range(M)]
            acc = 0.0
            for j in range(1, M + 1):
                acc = pc.add_up(acc, pc.mul_up(envelopes[j], residuals[j - 1]))
            value = pc.mul_up(scales[k], acc)
            if shift:
                value = pc.scale_bound_up(value, shift)
            unit = 2.0 ** shift
            digest = pc.envelope_key_digest(coefficients)
            digests.append(digest)
            adjoint.append({"column": k, "coefficients": [hx(x) for x in coefficients],
                            "envelopes": [hx(x) for x in envelopes], "residuals": [hx(x) for x in residuals],
                            "scale_abs": hx(scales[k]),
                            "envelope_key": {"degree": M, "extent_bits": hx(extent), "depth": pc.ADJOINT_DEPTH,
                                             "coefficients_sha256": digest, "proof_version": pc.PROOF_VERSION,
                                             "degree_limit": pc.DEGREE_LIMIT, "max_depth": pc.MAX_DEPTH}})
            terms.append({"column": k, "truncation": hx(rng.uniform(1e-12, 1e-11) * unit),
                          "coefficient": hx(rng.uniform(1e-12, 1e-11) * unit), "recurrence_adjoint": hx(value),
                          "summation": hx(rng.uniform(1e-17, 1e-16) * unit),
                          "normalization": hx(0.0 if shift == 0 else rng.uniform(1, 3) * 2.0 ** -1074),
                          "recurrence_majorant": hx(rng.uniform(1, 2))})
        fused_summation = rng.uniform(1e-17, 1e-16) * 2.0 ** shift
        total = fused_summation
        for k in range(5):
            for f in ("truncation", "coefficient", "summation"):
                total = pc.add_up(total, struct.unpack(">d", bytes.fromhex(terms[k][f]))[0])
        for f in ("recurrence_adjoint", "normalization"):
            for k in range(5):
                total = pc.add_up(total, struct.unpack(">d", bytes.fromhex(terms[k][f]))[0])
        identity = {"basis": "laguerre", "branch": "recurrence", "execution": "certified-enclosures",
                    "input": pc.registered_input(label), "degree": M, "laguerre_scale": hx(16.0), "beta": hx(beta),
                    "laguerre_extent": hx(extent), "h": hx(h),
                    "truncation_budget": hx(pc.registered_truncation_budget(label)),
                    "enclosure_lambda": hx(0.0), "enclosure_rho": hx(rho_enc), "enclosure_evidence": "gershgorin",
                    "operator_fingerprint": pc.operator_fingerprint(a, 0.0, rho_enc),
                    "normalization_shift": shift, "adjoint_proof_version": pc.PROOF_VERSION,
                    "adjoint_depth": pc.ADJOINT_DEPTH, "envelope_key_digests": digests}
        stamp = pc.canonical_identity_sha256(identity)
        for block in terms + adjoint:
            block["identity_sha256"] = stamp
        fused = [rng.uniform(-1, 1) * 2.0 ** shift for _ in range(N)]
        budget = 1.0e-6
        published_cases.append({
            "label": label, "n": N, "a": [[hx(x) for x in row] for row in a], "h": hx(h),
            "w": [[hx(x) for x in row] for row in w], "fused": [hx(x) for x in fused], "degree": M,
            "laguerre_scale": 16.0, "normalization_shift": shift, "budget": budget,
            "coverage": {"holds": True, "recomputed": total, "total": total}})
        components_cases.append({
            "label": label, "n": N, "a": [[hx(x) for x in row] for row in a], "h": hx(h),
            "w": [[hx(x) for x in row] for row in w], "input": pc.registered_input(label),
            "scales": [hx(s) for s in (pc.SCALES if same else [1.0] * 5)], "rho_nominal": hx(rho),
            "admission_budget": hx(budget),
            "report": {"degree": M, "laguerre_scale": hx(16.0), "normalization_shift": shift,
                       "fused": [hx(x) for x in fused], "laguerre_adjoint_total": hx(total),
                       "truncation_bound_exact_arithmetic": hx(1e-11), "evidence": "Gershgorin",
                       "admission": {"admitted": False, "reason": "synthetic"}},
            "parity": {"report_bitwise_equal": True, "counters_equal": True, "counters": {"poly_block_products": M}},
            "export": {"schema": pc.EXPORT_SCHEMA, "identity": identity, "identity_sha256": stamp, "terms": terms,
                       "assembly": {"identity_sha256": stamp, "fused_summation": hx(fused_summation),
                                    "laguerre_adjoint_total": hx(total)},
                       "adjoint": adjoint}})
        if label in pc.GATED:
            pp09_rows.append({"label": label, "derived_supplementary": {"truncation": "1.0", "coefficient": "1.0"}})
    published = {"schema": "synthetic", "cases": published_cases}
    pp09 = {"g3": {"rows": pp09_rows}}
    components = {"schema": pc.COMPONENTS_SCHEMA, "node": pc.NODE, "source_commit": "a" * 40, "tree_status": "",
                  "published": {"path": pc.PUBLISHED_PATH, "sha256": None}, "cases": components_cases}
    return published, pp09, components


def restamp(case: dict) -> None:
    """Recompute the identity stamp of a case after an identity edit (keeps the stamps consistent)."""
    stamp = pc.canonical_identity_sha256(case["export"]["identity"])
    e = case["export"]
    e["identity_sha256"] = stamp
    e["assembly"]["identity_sha256"] = stamp
    for block in e["terms"] + e["adjoint"]:
        block["identity_sha256"] = stamp


def by_label(doc: dict, label: str) -> dict:
    return next(c for c in doc["cases"] if c["label"] == label)


class CheckerTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.saved = (pc.PUBLISHED_SHA256, pc.PP09_SHA256, pc.SAMPLE_POINTS, pc.WORKERS)
        pc.SAMPLE_POINTS = 17
        pc.WORKERS = 1
        self.published, self.pp09, self.components = build()
        self.blob = Path(pc.__file__).read_bytes()
        self.status = ""

    def tearDown(self):
        pc.PUBLISHED_SHA256, pc.PP09_SHA256, pc.SAMPLE_POINTS, pc.WORKERS = self.saved
        self.tmp.cleanup()

    def run_checker(self, published_text: str | None = None, components_text: str | None = None,
                    pin: bool = True) -> tuple[int, dict]:
        pub = self.dir / "published.json"
        pub.write_text(published_text if published_text is not None else json.dumps(self.published))
        pp09 = self.dir / "pp09.json"
        pp09.write_text(json.dumps(self.pp09))
        if pin:
            pc.PUBLISHED_SHA256 = hashlib.sha256(pub.read_bytes()).hexdigest()
            pc.PP09_SHA256 = hashlib.sha256(pp09.read_bytes()).hexdigest()
        if components_text is None:
            comp = copy.deepcopy(self.components)
            comp["published"]["sha256"] = pc.PUBLISHED_SHA256
            components_text = json.dumps(comp)
        components = self.dir / "components.json"
        components.write_text(components_text)
        out = self.dir / "results.json"
        if out.exists():
            out.unlink()
        code = pc.main(["--published", str(pub), "--pp09", str(pp09), "--components", str(components),
                        "--output", str(out)], blob_at=lambda commit: self.blob,
                       tree_status=lambda: self.status, head=lambda: "b" * 40)
        return code, json.loads(out.read_text())

    def assert_invalid(self, **kwargs):
        code, result = self.run_checker(**kwargs)
        self.assertEqual((code, result["verdict"]), (2, "INVALID"), result.get("reasons"))
        return result

    def assert_fail(self, item: str, **kwargs):
        code, result = self.run_checker(**kwargs)
        self.assertEqual((code, result["verdict"]), (1, "FAIL"))
        self.assertFalse(result["gate"][item])
        return result

    # ------------------------------------------------------------------------------------------ PASS
    def test_valid_fixture_passes(self):
        code, result = self.run_checker()
        self.assertEqual((code, result["verdict"]), (0, "PASS"))
        self.assertTrue(all(result["gate"].values()))
        rep = result["reported"]
        case = rep["cases"]["n6-rho400-h0.1-same"]
        self.assertIn(case["bottleneck"], ("truncation", "coefficient", "recurrence_adjoint", "summation",
                                           "assembly"))
        self.assertIn("closability_floor_F", case)
        self.assertEqual(len(case["columns"][0]["l_j"]), M + 1)
        # The l_j are lower bounds of sup |z_j|: at x = 0, z_m = c_m, so l_m >= |c_m|.
        c_m = struct.unpack(">d", bytes.fromhex(
            by_label(self.components, "n6-rho400-h0.1-same")["export"]["adjoint"][0]["coefficients"][M]))[0]
        l_m = struct.unpack(">d", bytes.fromhex(case["columns"][0]["l_j_hex_rounded_down"][M]))[0]
        self.assertLessEqual(l_m, abs(c_m))
        self.assertGreaterEqual(l_m, abs(c_m) * (1 - 1e-15))

    def test_lower_bounds_match_a_scalar_backward_recurrence(self):
        coefficients = [0.5, -0.25, 0.125, 0.75]
        task = ([hx(x) for x in coefficients], hx(16.0), 17, 60)
        bounds = [n / d for n, d in pc.sampled_lower_bounds(task)]
        best = [0.0] * 4
        for i in range(17):
            x = 16.0 * i / 16
            z1 = z2 = 0.0
            for j in range(3, -1, -1):
                z = coefficients[j] + (2 * j + 1 - x) / (j + 1) * z1 - (j + 1) / (j + 2) * z2
                best[j] = max(best[j], abs(z))
                z2, z1 = z1, z
        for got, want in zip(bounds, best):
            self.assertAlmostEqual(got, want, delta=1e-12 * max(1.0, want))

    def test_directed_emulation(self):
        from fractions import Fraction
        rng = random.Random(3)
        for _ in range(200):
            a, b = rng.uniform(-1, 1) * 10 ** rng.randint(-5, 5), rng.uniform(-1, 1) * 10 ** rng.randint(-5, 5)
            for op, exact in ((pc.add_up, Fraction(a) + Fraction(b)), (pc.mul_up, Fraction(a) * Fraction(b))):
                up = op(a, b)  # the smallest binary64 >= the exact value
                self.assertGreaterEqual(Fraction(up), exact)
                self.assertLess(Fraction(math.nextafter(up, -math.inf)), exact)
        self.assertEqual(pc.add_up(1.0, 2.0), 3.0)
        self.assertEqual(pc.mul_up(0.5, 4.0), 2.0)
        tiny = 2.0 ** -1074
        self.assertEqual(pc.mul_up(tiny, 1.0), 2 * tiny)  # unconditional step below 1e-289
        self.assertEqual(pc.scale_bound_up(0.5, -1073), tiny)  # exact
        self.assertEqual(pc.scale_bound_up(0.5, -1075), tiny)  # below the subnormal range
        self.assertEqual(pc.scale_bound_up(0.75, -1074), 2 * tiny)  # rounds to 1 ulp, inexact -> successor
        self.assertEqual(pc.scale_bound_up(0.75, 997), 0.75 * 2.0 ** 997)
        self.assertEqual(pc.scale_bound_up(3.0, -1076), 2 * tiny)  # 0.75 ulp rounds to 1 ulp, inexact -> 2

    # --------------------------------------------------------------------------------------- INVALID
    def test_invalid_non_strict_json(self):
        text = json.dumps(self.components).replace('"n": 2', '"n": NaN', 1)
        self.assert_invalid(components_text=text)

    def test_invalid_published_pin(self):
        pc.PUBLISHED_SHA256 = "0" * 64
        pc.PP09_SHA256 = hashlib.sha256(json.dumps(self.pp09).encode()).hexdigest()
        self.assert_invalid(pin=False)

    def test_invalid_missing_and_extra_keys(self):
        del self.components["cases"][0]["export"]["terms"][1]["summation"]
        self.assert_invalid()
        self.components = build()[2]
        self.components["cases"][2]["report"]["extra"] = 1
        self.assert_invalid()

    def test_invalid_duplicate_and_missing_cases(self):
        self.components["cases"].append(copy.deepcopy(self.components["cases"][0]))
        self.assert_invalid()
        self.components = build()[2]
        self.components["cases"].pop()
        self.assert_invalid()

    def test_invalid_hex(self):
        case = self.components["cases"][1]
        case["export"]["terms"][0]["truncation"] = case["export"]["terms"][0]["truncation"].upper()
        self.assert_invalid()
        self.components = build()[2]
        self.components["cases"][1]["export"]["adjoint"][2]["envelopes"][1] = "7ff0000000000000"
        self.assert_invalid()

    def test_invalid_binding(self):
        for mutate in (
            lambda c: c["a"][0].__setitem__(1, hx(0.125)),
            lambda c: c["w"][3].__setitem__(0, hx(0.125)),
            lambda c: c.__setitem__("h", hx(0.5)),
            lambda c: c["report"]["fused"].__setitem__(0, hx(0.125)),
            lambda c: c["report"].__setitem__("degree", M + 1),
            lambda c: c["report"].__setitem__("normalization_shift", 3),
            lambda c: c["report"].__setitem__("laguerre_adjoint_total", hx(1.0)),
        ):
            self.components = build()[2]
            mutate(by_label(self.components, "n12-rho400-h0.1-same"))
            self.assert_invalid()

    def test_invalid_registered_construction(self):
        case = by_label(self.components, "large-1e300")
        case["export"]["identity"]["truncation_budget"] = hx(1.0e-10)  # the literal reading, not R-NEXT-04's
        restamp(case)
        self.assert_invalid()

    def test_invalid_stamp(self):
        case = by_label(self.components, "n20-rho400-h0.1-distinct")
        case["export"]["terms"][4]["identity_sha256"] = "f" * 64
        self.assert_invalid()
        self.components = build()[2]
        case = by_label(self.components, "n20-rho400-h0.1-distinct")
        case["export"]["identity"]["beta"] = hx(1.0)  # identity edited, stamps left
        self.assert_invalid()

    def test_invalid_shared_identity(self):
        first = by_label(self.published, "n6-rho400-h0.1-distinct")
        second = by_label(self.published, "n6-rho400-h0.01-distinct")
        for key in ("a", "h", "w", "fused", "degree", "coverage", "budget"):
            second[key] = copy.deepcopy(first[key])
        c1 = by_label(self.components, "n6-rho400-h0.1-distinct")
        c2 = by_label(self.components, "n6-rho400-h0.01-distinct")
        index = self.components["cases"].index(c2)
        clone = copy.deepcopy(c1)
        clone["label"] = c2["label"]
        self.components["cases"][index] = clone
        result = self.assert_invalid()
        self.assertTrue(any("shares its identity" in r for r in result["reasons"]))

    def test_invalid_provenance(self):
        self.blob = b"another checker"
        self.assert_invalid()
        self.blob = Path(pc.__file__).read_bytes()
        self.components["tree_status"] = " M crates/rodas5p-core/src/polynomial_action.rs\n"
        self.assert_invalid()
        self.components["tree_status"] = ""
        self.status = " M tools/py01_laguerre_component_check.py\n"
        self.assert_invalid()

    # ------------------------------------------------------------------------------------------ FAIL
    def test_fail_completeness(self):
        by_label(self.components, "n6-rho400-h0.1-same")["export"]["terms"][3]["coefficient"] = None
        self.assert_fail("1_completeness")
        self.components = build()[2]
        by_label(self.components, "subnormal-1e-310")["export"]["terms"][0]["summation"] = hx(-1e-300)
        self.assert_fail("1_completeness")
        self.components = build()[2]
        by_label(self.components, "n6-rho400-h0.1-same")["export"]["adjoint"][1]["residuals"].pop()
        self.assert_fail("1_completeness")

    def test_fail_enclosure_in_window(self):
        case = by_label(self.components, "n12-rho400-h0.1-distinct")
        total = struct.unpack(">d", bytes.fromhex(case["report"]["laguerre_adjoint_total"]))[0]
        lower = total * (1 - 1e-15)
        case["report"]["laguerre_adjoint_total"] = hx(lower)
        case["export"]["assembly"]["laguerre_adjoint_total"] = hx(lower)
        by_label(self.published, "n12-rho400-h0.1-distinct")["coverage"]["total"] = lower
        result = self.assert_fail("2_enclosure")
        self.assertTrue(result["gate"]["1_completeness"] and result["gate"]["3_source_binding"])

    def test_fail_enclosure_out_of_window(self):
        case = by_label(self.components, "large-1e300")
        total = struct.unpack(">d", bytes.fromhex(case["report"]["laguerre_adjoint_total"]))[0]
        lower = total * (1 - 1e-9)
        case["report"]["laguerre_adjoint_total"] = hx(lower)
        case["export"]["assembly"]["laguerre_adjoint_total"] = hx(lower)
        by_label(self.published, "large-1e300")["coverage"]["total"] = lower
        self.assert_fail("2_enclosure")
        # Within the amended allowance it passes.
        self.published, self.pp09, self.components = build()
        case = by_label(self.components, "subnormal-1e-310")
        total = struct.unpack(">d", bytes.fromhex(case["report"]["laguerre_adjoint_total"]))[0]
        lower = total - 20 * 2.0 ** -1074
        case["report"]["laguerre_adjoint_total"] = hx(lower)
        case["export"]["assembly"]["laguerre_adjoint_total"] = hx(lower)
        by_label(self.published, "subnormal-1e-310")["coverage"]["total"] = lower
        code, result = self.run_checker()
        self.assertEqual((code, result["verdict"]), (0, "PASS"))

    def test_fail_source_binding(self):
        case = by_label(self.components, "n20-rho400-h0.1-same")
        envelopes = case["export"]["adjoint"][2]["envelopes"]
        envelopes[2] = hx(2 * struct.unpack(">d", bytes.fromhex(envelopes[2]))[0])
        result = self.assert_fail("3_source_binding")
        self.assertTrue(result["gate"]["2_enclosure"])
        self.published, self.pp09, self.components = build()
        case = by_label(self.components, "n20-rho400-h0.1-same")
        case["export"]["adjoint"][4]["coefficients"][0] = hx(0.123)
        self.assert_fail("3_source_binding")
        self.published, self.pp09, self.components = build()
        case = by_label(self.components, "subnormal-1e-310")
        case["export"]["adjoint"][0]["envelope_key"]["depth"] = 4
        self.assert_fail("3_source_binding")


if __name__ == "__main__":
    unittest.main()
