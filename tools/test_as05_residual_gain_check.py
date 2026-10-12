#!/usr/bin/env python3
"""Synthetic mutation tests for tools/as05_residual_gain_check.py (research node AS05).

Every fixture is a synthetic RUNS.json and RECEIPTS.json written to a temporary directory: the registered inputs are
regenerated with the checker's own generators, the research GainWitness2 enclosures of the W-formation family are
emulated here (round-to-nearest plus one-ulp widening), and every Verified record is a generous enclosure of the exact
Fraction error. The committed NATIVE.json is the retained input; git is replaced by a fake repository; no Rust code
runs and no committed research file is written.
- a valid fixture that PASSes (exit 0);
- each registered validity condition violated -> INVALID (exit 2);
- each gate item 1-6 violated on its own -> FAIL (exit 1).

Run: python3 -m unittest discover -s tools -p test_as05_residual_gain_check.py
"""

from __future__ import annotations

import copy
import json
import math
import shutil
import struct
import sys
import tempfile
import unittest
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import as05_residual_gain_check as ck  # noqa: E402

NATIVE = ROOT / "research/reaudit_accuracy_speed_20261010/NATIVE.json"
COMMIT = "0123456789abcdef0123456789abcdef01234567"


def hx(x: float) -> str:
    return struct.pack(">d", x).hex()


nu, nd = (lambda x: math.nextafter(x, math.inf)), (lambda x: math.nextafter(x, -math.inf))


def research_weighted(w, b, x):
    """Emulation of GainWitness2 (unit scales): round to nearest, widen one ulp; None if rejected."""
    def out(lo, hi):
        return nd(lo), nu(hi)

    def add(a, c):
        return out(a[0] + c[0], a[1] + c[1])

    def sub(a, c):
        return out(a[0] - c[1], a[1] - c[0])

    def mul(a, c):
        v = [a[0] * c[0], a[0] * c[1], a[1] * c[0], a[1] * c[1]]
        return out(min(v), max(v))

    def div(a, c):
        v = [a[0] / c[0], a[0] / c[1], a[1] / c[0], a[1] / c[1]]
        return out(min(v), max(v))

    p = [[(v, v) for v in row] for row in w]
    det = sub(mul(p[0][0], p[1][1]), mul(p[0][1], p[1][0]))
    if det[0] <= 0 <= det[1]:
        return None
    neg = (lambda t: (-t[1], -t[0]))
    inv = [[div(p[1][1], det), div(neg(p[0][1]), det)], [div(neg(p[1][0]), det), div(p[0][0], det)]]
    r = [sub((b[i], b[i]), add(mul(p[i][0], (x[0], x[0])), mul(p[i][1], (x[1], x[1])))) for i in range(2)]
    corr = [add(mul(inv[i][0], r[0]), mul(inv[i][1], r[1])) for i in range(2)]
    return [div(corr[i], (1.0, 1.0)) for i in range(2)]


def up(q: F) -> float:
    """A binary64 >= q >= 0, generous."""
    v = float(q)
    return nu(nu(v * (1 + 1e-12))) if v else 0.0


class Model:
    def __init__(self):
        self.native = json.loads(NATIVE.read_text())
        self.gamma = ck.gamma_from_snapshot(ROOT)
        self.members = ck.w_formation_inputs(self.gamma)
        self.research = [research_weighted(m["w_rep"], m["rhs"], m["candidate"]) for m in self.members]
        self.decision = [ck.discriminates(m, r) for m, r in zip(self.members, self.research)]
        self.chosen = [m for m, d in zip(self.members, self.decision) if d][:4]
        self.lognorm = {}
        for n, pes, hs in ((3, (0.0, 10.0), (1e-3, 1e-1)), (4, (0.0, 10.0), (1e-3, 1e-1)),
                           (8, (0.0, 10.0), (1e-3, 1e-1))):
            for pe in pes:
                for h in hs:
                    cid = f"lognorm_n{n}_pe{ck.rust_display(pe)}_a{ck.rust_exp(h)}"
                    self.lognorm[cid] = self.lognorm_input(ck.lognorm_j(n, pe), h)
        self.lognorm["lognorm_negative_shift"] = self.lognorm_input(ck.lognorm_j(3, 0.0), -1e-3)
        self.lognorm["lognorm_nonpositive_denominator"] = self.lognorm_input(
            [[10.0 if i == k else 0.0 for k in range(3)] for i in range(3)], 1.0)
        self.cases = ck.registered_cases(self.native, self.gamma, self.chosen, self.lognorm)

    @staticmethod
    def lognorm_input(j, h):
        n = len(j)
        metric = [min(s) for s in ck.faithful_metric(n)]
        w = [[F(int(i == k)) - F(h) * F(j[i][k]) for k in range(n)] for i in range(n)]
        base = [float(v) for v in ck.solve(w, [F(1)] * n)]
        return metric, base

    def verified(self, case, route, exact_like=None):
        t = case["target"]
        n = len(case["rhs"])
        errs = [ck.exact_errors(t, case["rhs"], case["candidate"], corner) for corner in ck.corners(t)]
        wmax = [max(abs(e[k] / F(t["metric"][k])) for e in errs) for k in range(n)]
        cmax = [max(abs(e[k]) for e in errs) for k in range(n)]
        m = up(max(wmax))
        lognorm = route == "lognorm"
        gain = None if lognorm else up(ck.exact_gain(t, t["metric"]))
        return {
            "route": route,
            "residual_hex": [[hx(-1.0), hx(1.0)] for _ in range(n)],
            "correction_hex": None if lognorm else [[hx(-up(c)), hx(up(c))] for c in cmax],
            "weighted_correction_hex": None if lognorm else [[hx(-up(c)), hx(up(c))] for c in wmax],
            "weighted_inf_upper_hex": hx(m), "wrms_upper_hex": hx(m), "gain_hex": None if gain is None else hx(gain),
            "lognorm": {"shift_hex": [hx(0.1), hx(0.1)], "mu_up_hex": hx(-1.0), "mu_source": "interval-cholesky",
                        "denominator_lower_hex": hx(1.1)} if lognorm else None,
            "setup_flops": 100, "verification_flops": 50, "transport_factor_hex": None,
            "bound_operator_sha256": ck.operator_sha256(t), "bound_metric_hex": [hx(v) for v in t["metric"]],
            "bound_model_epoch": t["model_epoch"],
        }

    def evidence(self, cid, case):
        g = case["group"]
        if g == "A_cell":
            raw = next(x for x in self.native["gain_study"]["cells"] if x["id"] == cid)
            res = ck.research_expected(raw)
            t = case["target"]
            return {"kind": "verified", "rejection": None, "estimate": None, "verified": {
                "route": "represented_dense2", "residual_hex": res["residual_hex"],
                "correction_hex": res["correction_hex"], "weighted_correction_hex": res["weighted_correction_hex"],
                "weighted_inf_upper_hex": res["weighted_inf_upper_hex"], "wrms_upper_hex": res["wrms_upper_hex"],
                "gain_hex": res["gain_hex"], "lognorm": None, "setup_flops": 122, "verification_flops": 92,
                "transport_factor_hex": None, "bound_operator_sha256": ck.operator_sha256(t),
                "bound_metric_hex": [hx(v) for v in t["metric"]], "bound_model_epoch": t["model_epoch"]}}
        if g == "A_invalid":
            raw = next(x for x in self.native["gain_study"]["invalid_controls"] if x["id"] == cid)
            return unavailable(ck.RESEARCH_CLASS[raw["error"]])
        if g in ck.C_CLASS:
            return unavailable(ck.C_CLASS[g])
        if g == "C_offblock":
            return unavailable("UndeclaredStructure" if cid == "dense4_route_a" else "OffBlockCoupling")
        if g in ("C_assembly", "C_nu"):
            src = "UndeclaredAssembly" if g == "C_assembly" else "ProjectedNu"
            return {"kind": "estimated", "rejection": None, "estimate": {"source": src, "value_hex": hx(0.5)},
                    "verified": None}
        if g == "D_lognorm_negative":
            return unavailable("NegativeShift" if cid == "lognorm_negative_shift" else "NonpositiveDenominator")
        route = {"D_blocks": "represented_blocks", "D_lognorm": "lognorm", "D_transport": "represented_dense2"}.get(
            g, "shifted_enclosure")
        return {"kind": "verified", "rejection": None, "estimate": None, "verified": self.verified(case, route)}

    def runs(self):
        fam = []
        for m, r, d in zip(self.members, self.research, self.decision):
            fam.append({"id": m["id"], "h_hex": hx(m["h"]), "k_hex": hx(m["k"]), "gamma_hex": hx(m["gamma"]),
                        "lambda_hex": hx(m["lambda"]), "j_hex": [[hx(v) for v in row] for row in m["j"]],
                        "w_rep_hex": [[hx(v) for v in row] for row in m["w_rep"]],
                        "rhs_hex": [hx(v) for v in m["rhs"]], "candidate_hex": [hx(v) for v in m["candidate"]],
                        "research": {"status": "admitted", "correction_hex": [[hx(a), hx(b)] for a, b in r],
                                     "weighted_correction_hex": [[hx(a), hx(b)] for a, b in r],
                                     "wrms_upper_hex": hx(1.0)},
                        "discriminates": d, "fixture": m in self.chosen})
        cases = []
        for cid, case in self.cases.items():
            g = case["group"]
            research = None
            if g in ("A_cell", "A_invalid"):
                key = "cells" if g == "A_cell" else "invalid_controls"
                research = ck.research_expected(next(x for x in self.native["gain_study"][key] if x["id"] == cid))
            note = None
            if g in ("D_lognorm", "D_lognorm_negative"):
                note = {"lu_base_hex": [hx(v) for v in self.lognorm[cid][1]]}
            comparator = None
            if g in ck.D_GROUPS:
                comparator = {"source": "DenseLuComparator", "estimate_hex": hx(1e-9),
                              "nominal_flops": 2.0 / 3.0 * len(case["rhs"]) ** 3 + 2.0 * len(case["rhs"]) ** 2,
                              "rejection": None}
            cases.append({"id": cid, "group": g, "route": case["route"], "witness": ck.binding_hex(case["witness"]),
                          "target": ck.binding_hex(case["target"]), "rhs_hex": [hx(v) for v in case["rhs"]],
                          "candidate_hex": [hx(v) for v in case["candidate"]],
                          "evidence": self.evidence(cid, case), "comparator": comparator, "research": research,
                          "note": note})
        return {"schema": ck.RUNS_SCHEMA, "source": {"commit": COMMIT, "tree_status": ""},
                "gain_witness_sha256": ck.PINS["gain_witness"][1], "gamma_hex": hx(self.gamma), "model_epoch": 7,
                "w_formation_family": fam, "fixtures": [m["id"] for m in self.chosen], "cases": cases}


def unavailable(cls):
    return {"kind": "unavailable", "rejection": cls, "estimate": None, "verified": None}


def receipts():
    suite = []
    for name in ("unittests src/lib.rs", "tests/pp12_lognorm_decay.rs", "tests/pp12b_chain_symmetrizer.rs",
                 "tests/int05_nonnormal_contracts.rs", "tests/rev02_nonnormal_contracts.rs",
                 "tests/safe_enclosure_probes.rs", "tests/residual_gain_contract.rs"):
        suite += [f"Running {name} (/t/deps/x)", "test a ... ok",
                  "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.1s"]
    doc = ["Doc-tests rodas5p_core", "test crates/rodas5p-core/src/residual_gain.rs - residual_gain (line 44) ... ok"]
    doc += [f"test crates/rodas5p-core/src/residual_gain.rs - residual_gain (line {n}) - compile fail ... ok"
            for n in (61, 67, 72, 77)]
    doc += ["test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.3s"]
    cmds = [{"name": "contract_tests", "argv": ["cargo"], "exit_code": 0, "lines": suite[-3:]},
            {"name": "doc_residual_gain", "argv": ["cargo"], "exit_code": 0, "lines": doc},
            {"name": "rodas5p_core_all_targets", "argv": ["cargo"], "exit_code": 0, "lines": suite}]
    return {"schema": ck.RECEIPTS_SCHEMA, "commit": COMMIT, "tree_status_before": "", "tree_status_after": "",
            "commands": cmds}


class FakeRepo(ck.Repo):
    def __init__(self):
        super().__init__(ROOT)
        self.blob = Path(ck.__file__).read_bytes()
        self.status = [("A", "crates/rodas5p-core/tests/residual_gain_contract.rs")]
        self.refs = ["crates/rodas5p-core/src/lib.rs", "crates/rodas5p-core/src/residual_gain.rs"]

    def show(self, commit, path):
        return self.blob if commit == COMMIT and path == ck.CHECKER_PATH else None

    def diff_status(self, base, commit, path):
        return self.status

    def grep(self, commit, pattern):
        return self.refs


_MODEL = None
_BASE = None


def model() -> Model:
    global _MODEL
    if _MODEL is None:
        _MODEL = Model()
    return _MODEL


def base_runs():
    global _BASE
    if _BASE is None:
        _BASE = model().runs()
    return copy.deepcopy(_BASE)


class CheckerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.runs = base_runs()
        self.receipts = receipts()
        self.repo = FakeRepo()
        self.source = None
        self.native = NATIVE

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def run_checker(self, raw_runs: bytes | None = None):
        runs_path = self.tmp / "RUNS.json"
        runs_path.write_bytes(raw_runs if raw_runs is not None else json.dumps(self.runs).encode())
        (self.tmp / "RECEIPTS.json").write_text(json.dumps(self.receipts))
        out = self.tmp / "RESULTS.json"
        if out.exists():
            out.unlink()
        code = ck.main(["--runs", str(runs_path), "--retained", str(self.native), "--output", str(out)],
                       git=self.repo, source_path=self.source)
        return code, json.loads(out.read_text())

    def case(self, cid):
        return next(c for c in self.runs["cases"] if c["id"] == cid)

    def assertInvalid(self, raw=None):
        code, report = self.run_checker(raw)
        self.assertEqual((code, report["verdict"]), (2, "INVALID"), report.get("gate"))
        return report

    def assertFails(self, item):
        code, report = self.run_checker()
        self.assertEqual((code, report["verdict"]), (1, "FAIL"))
        failing = [k for k, g in report["gate"].items() if not g["pass"]]
        self.assertEqual(failing, [item])
        return report

    # ------------------------------------------------------------------ valid
    def test_valid_fixture_passes(self):
        code, report = self.run_checker()
        self.assertEqual((code, report["verdict"]), (0, "PASS"), json.dumps(report.get("reasons") or {
            k: g["failures"] for k, g in report["gate"].items()})[:2000])
        self.assertEqual(len(report["fixtures"]), 4)

    def test_cli_immutable_output(self):
        out = self.tmp / "RESULTS.json"
        out.write_text("{}")
        with self.assertRaises(SystemExit):
            ck.main(["--runs", str(self.tmp / "RUNS.json"), "--retained", str(NATIVE), "--output", str(out)],
                    git=self.repo)

    # ---------------------------------------------------------------- INVALID
    def test_strict_json(self):
        self.assertInvalid(b'{"schema": NaN}')

    def test_duplicate_key(self):
        self.assertInvalid(b'{"schema": 1, "schema": 2}')

    def test_native_pin(self):
        copy_path = self.tmp / "NATIVE.json"
        copy_path.write_text(NATIVE.read_text() + " ")
        self.native = copy_path
        self.assertInvalid()

    def test_bad_hex(self):
        self.case("control_symmetric")["evidence"]["verified"]["wrms_upper_hex"] = "XYZ"
        self.assertInvalid()

    def test_missing_case(self):
        self.runs["cases"].pop(5)
        self.assertInvalid()

    def test_duplicated_case(self):
        self.runs["cases"].append(copy.deepcopy(self.runs["cases"][0]))
        self.assertInvalid()

    def test_input_mismatch(self):
        c = self.case("holdout_k7_p0")
        c["candidate_hex"][0] = hx(nu(struct.unpack(">d", bytes.fromhex(c["candidate_hex"][0]))[0]))
        self.assertInvalid()

    def test_binding_mismatch(self):
        self.case("grid_k1_s0_p0_epoch_8")["target"]["model_epoch"] = 9
        self.assertInvalid()

    def test_research_reference_mismatch(self):
        self.case("grid_k1_s0_p0")["research"]["wrms_upper_hex"] = hx(1.0)
        self.assertInvalid()

    def test_discrimination_flag_mismatch(self):
        self.runs["w_formation_family"][0]["discriminates"] = not self.runs["w_formation_family"][0]["discriminates"]
        self.assertInvalid()

    def test_fewer_than_four_discriminate(self):
        for row in self.runs["w_formation_family"]:
            row["research"]["weighted_correction_hex"] = [[hx(-1e300), hx(1e300)]] * 2
        self.assertInvalid()

    def test_dirty_tree(self):
        self.runs["source"]["tree_status"] = " M x.rs\n"
        self.assertInvalid()

    def test_dirty_receipt(self):
        self.receipts["tree_status_after"] = "?? y\n"
        self.assertInvalid()

    def test_checker_blob_differs(self):
        self.repo.blob = b"# another checker\n"
        self.assertInvalid()

    def test_checker_postdates_run(self):
        self.runs["source"]["commit"] = "f" * 40
        self.receipts["commit"] = "f" * 40
        self.assertInvalid()

    def test_receipt_commit_differs(self):
        self.receipts["commit"] = "e" * 40
        self.assertInvalid()

    def test_gamma_mismatch(self):
        self.runs["gamma_hex"] = hx(0.2)
        self.assertInvalid()

    def test_unfaithful_lognorm_metric(self):
        c = self.case("lognorm_n3_pe0_a1e-3")
        for key in ("witness", "target"):
            c[key]["metric_hex"][0] = hx(1.5)
        self.assertInvalid()

    def test_unknown_rejection(self):
        self.case("grid_k1_s0_p0_metric_next_up")["evidence"]["rejection"] = "Whatever"
        self.assertInvalid()

    # ------------------------------------------------------------------- FAIL
    def test_item1_cell_unavailable(self):
        # An unavailable retained cell also has no certificate to compare in item 2.
        self.case("grid_k1_s0_p0")["evidence"] = unavailable("ArithmeticOverflow")
        code, report = self.run_checker()
        self.assertEqual((code, report["verdict"]), (1, "FAIL"))
        failing = [k for k, g in report["gate"].items() if not g["pass"]]
        self.assertEqual(failing, ["1_retained_oracle", "2_parity"])

    def test_item1_bound_too_small(self):
        v = self.case("holdout_k65536_p1")["evidence"]["verified"]
        v["wrms_upper_hex"] = hx(1e-300)
        self.assertFails("1_retained_oracle")

    def test_item1_invalid_class(self):
        self.case("invalid_nonfinite_b")["evidence"] = unavailable("NonfiniteCandidate")
        self.assertFails("1_retained_oracle")

    def test_item1_overlap_exception_unavailable_passes(self):
        self.case(ck.OVERLAP_EXCEPTION)["evidence"] = unavailable("ArithmeticOverflow")
        code, report = self.run_checker()
        self.assertEqual(code, 0)

    def test_item2_parity(self):
        v = self.case("grid_k1024_s1_p0")["evidence"]["verified"]
        lo = struct.unpack(">d", bytes.fromhex(v["residual_hex"][0][0]))[0]
        v["residual_hex"][0][0] = hx(nd(lo))
        self.assertFails("2_parity")

    def test_item3_wrong_class(self):
        self.case("grid_k1_s0_p0_metric_next_up")["evidence"] = unavailable("StaleEpoch")
        self.assertFails("3_negatives_refused")

    def test_item3_nu_verified(self):
        good = copy.deepcopy(self.case("blocks_n4")["evidence"])
        good["verified"]["residual_hex"] = good["verified"]["residual_hex"][:2]
        good["verified"]["correction_hex"] = good["verified"]["correction_hex"][:2]
        good["verified"]["weighted_correction_hex"] = good["verified"]["weighted_correction_hex"][:2]
        good["verified"]["bound_metric_hex"] = good["verified"]["bound_metric_hex"][:2]
        self.case("nu_k1024")["evidence"] = good
        self.assertFails("3_negatives_refused")

    def test_item3_offblock_estimated(self):
        self.case("offblock_r0c2_one")["evidence"] = {"kind": "estimated", "rejection": None, "verified": None,
                                                       "estimate": {"source": "ProjectedNu", "value_hex": hx(1.0)}}
        self.assertFails("3_negatives_refused")

    def test_item4_false_certificate(self):
        v = self.case("blocks_n6")["evidence"]["verified"]
        v["wrms_upper_hex"] = hx(1e-300)
        self.assertFails("4_no_false_certificate")

    def test_item4_corner_violation(self):
        # Tighten a delta case to the exact centre (delta = 0) error: the 16 corners fall outside.
        cid = next(c for c in self.runs["cases"] if c["group"] == "D_shifted_delta")["id"]
        case = model().cases[cid]
        e = ck.exact_errors(case["target"], case["rhs"], case["candidate"])
        tight = [[hx(nd(nd(float(x)))), hx(nu(nu(float(x))))] for x in e]
        v = self.case(cid)["evidence"]["verified"]
        v["weighted_correction_hex"] = tight
        v["correction_hex"] = tight
        self.assertFails("4_no_false_certificate")

    def test_item4_binding(self):
        self.case("blocks_n3")["evidence"]["verified"]["bound_model_epoch"] = 8
        self.assertFails("4_no_false_certificate")

    def test_item5_pub_field(self):
        src = (ROOT / ck.SOURCE_PATH).read_text().replace(
            "pub struct LinearTargetId {\n    kind:", "pub struct LinearTargetId {\n    pub kind:")
        self.assertIn("pub kind:", src)
        self.source = self.tmp / "residual_gain.rs"
        self.source.write_text(src)
        self.assertFails("5_authority_types")

    def test_item5_deserialize(self):
        src = (ROOT / ck.SOURCE_PATH).read_text().replace(
            "#[derive(Clone, Debug, Serialize)]\npub struct VerifiedCorrection",
            "#[derive(Clone, Debug, Serialize, serde::Deserialize)]\npub struct VerifiedCorrection")
        self.assertIn("Deserialize)]\npub struct VerifiedCorrection", src)
        self.source = self.tmp / "residual_gain.rs"
        self.source.write_text(src)
        self.assertFails("5_authority_types")

    def test_item5_compile_fail_failed(self):
        doc = self.receipts["commands"][1]
        doc["lines"][2] = doc["lines"][2].replace("... ok", "... FAILED")
        self.assertFails("5_authority_types")

    def test_item6_suite_failed(self):
        suite = self.receipts["commands"][2]["lines"]
        i = suite.index("Running tests/pp12_lognorm_decay.rs (/t/deps/x)")
        suite[i + 2] = "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out"
        self.assertFails("6_non_regression_isolation")

    def test_item6_missing_required_binary(self):
        suite = self.receipts["commands"][2]["lines"]
        i = suite.index("Running tests/pp12b_chain_symmetrizer.rs (/t/deps/x)")
        self.receipts["commands"][2]["lines"] = suite[:i] + suite[i + 3:]
        self.assertFails("6_non_regression_isolation")

    def test_item6_modified_test_file(self):
        self.repo.status = self.repo.status + [("M", "crates/rodas5p-core/tests/pp12_lognorm_decay.rs")]
        self.assertFails("6_non_regression_isolation")

    def test_item6_outside_reference(self):
        self.repo.refs = self.repo.refs + ["crates/rodas5p-integrators/src/rodas5p_fast.rs"]
        self.assertFails("6_non_regression_isolation")


if __name__ == "__main__":
    unittest.main()
