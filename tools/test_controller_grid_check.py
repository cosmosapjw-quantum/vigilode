#!/usr/bin/env python3
"""Synthetic tests for tools/controller_grid_check.py (research node CT01).

Every fixture is a synthetic RUNS.json written to a temporary directory (the committed NATIVE.json is the
reference input; no solver is run and no committed research file is written):
- a valid fixture that PASSes (exit 0);
- each registered validity condition violated -> INVALID (exit 2);
- each gate item 1-5 violated on its own -> FAIL (exit 1), the other items still passing.

Run: python3 -m unittest discover -s tools -p test_controller_grid_check.py
"""

from __future__ import annotations

import json
import math
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
import controller_grid_check as cg  # noqa: E402
import evidence_schema_v2 as es  # noqa: E402

NATIVE = ROOT / "research/stiff_native_benchmark_20261001/NATIVE.json"
NATIVE_REFS = {p: json.loads(NATIVE.read_text())["references"][p]["final_state"] for p in cg.PROBLEMS}


def hx(x: float) -> str:
    return struct.pack(">d", x).hex()


class Model:
    """Synthetic run model: per arm, the error and the attempt/rejection counts of a cell; hooks override."""

    def __init__(self):
        self.vdp_work = 0.8          # PREDcap2/I attempts on van der Pol
        self.vdp_rejection = 0.05    # PREDcap2 rejection fraction on van der Pol (I: 0.2)
        self.work_override = {}      # (problem, seed) -> PREDcap2/I attempts
        self.error_factor = {}       # problem -> PREDcap2/I error
        self.cell_error = {}         # (problem, seed, q) -> PREDcap2 err/rtol
        self.failing = set()         # (problem, seed, q): PREDcap2 fails
        self.check_e = 1.0e-13       # relative perturbation of the check run
        self.clipped = 5

    def cell(self, problem, seed, q, arm):
        rtol = cg.quarter_decade(q)
        s = cg.SEEDS.index(seed)
        err_i = 0.5 * rtol * (1.0 + 0.1 * s)
        att_i = round(100.0 * rtol ** -0.2 * (1.0 + 0.01 * s))
        if arm == "I":
            return err_i, att_i, round(0.2 * att_i), True
        err, att, frac = err_i, att_i, 0.2
        if problem == cg.VDP:
            att, frac = round(self.vdp_work * att_i), self.vdp_rejection
        att = round(att * self.work_override.get((problem, seed), 1.0))
        err = err * self.error_factor.get(problem, 1.0)
        if (problem, seed, q) in self.cell_error and arm == "PREDcap2":
            err = self.cell_error[(problem, seed, q)] * rtol
        ok = not (arm == "PREDcap2" and (problem, seed, q) in self.failing)
        return err, att, round(frac * att), ok


_STATE_CACHE: dict = {}


def states_for(problem: str, e: float):
    """40 grid states r (1 + e) (as hex, one shared list) and their grid error."""
    key = (problem, e)
    if key not in _STATE_CACHE:
        ref = NATIVE_REFS[problem]
        y = [x * (1.0 + e) for x in ref]
        err = cg.point_error(y, ref)
        _STATE_CACHE[key] = ([hx(v) for v in y], err)
    return _STATE_CACHE[key]


def run_row(problem, arm, rtol, seed, err_rel, attempts, rejected, success, clipped, partial_points=10):
    hexes, err = states_for(problem, err_rel)
    times = [hx(t) for t in cg.grid_times(problem)]
    n = cg.GRID_POINTS if success else partial_points
    counters = {k: 0 for k in es.COUNTERS_DENSE}
    counters.update(accepted_steps=attempts - rejected, rejected_steps=rejected, direct_factorizations=attempts)
    return {
        "problem": problem, "arm": arm, "rtol": rtol, "atol": rtol * cg.ATOL_SCALE[problem], "seed": seed,
        "ok": True, "success": success, "message": "success" if success else "maximum step count or minimum step reached",
        "attempts": attempts, "accepted": attempts - rejected, "rejected": rejected, "jacobian_reuses": rejected,
        "internal_steps": attempts - rejected, "output_clipped_steps": clipped, "rhs_evaluations": 6 * attempts,
        "lu_factorizations": attempts, "jacobian_builds": attempts - rejected, "linear_solve_failures": 0,
        "nonfinite_step_failures": 0, "counters": counters, "grid_t": times[:n], "grid_states": [hexes] * n,
        "telemetry": {"clipped_landings": clipped, "sliver_landings": 1, "sliver_landings_while_rejection_pending": 0,
                      "informative_clipped_landings_after_rejection": 1,
                      "zero_error_accepts_while_rejection_pending": 0, "accepted_next_request_exceeds_trial": 2,
                      "updates": attempts, "rejection_pending_at_end": False},
        "grid_error": err if success else None, "endpoint_error": err if success else None,
    }


def make_doc(model: Model) -> dict:
    references = {}
    for p in cg.PROBLEMS:
        ref = run_row(p, "I", cg.REFERENCE_RTOL, cg.REFERENCE_SEED, 0.0, 5000, 10, True, 40)
        check = run_row(p, "I", cg.CHECK_RTOL, cg.REFERENCE_SEED, model.check_e, 4000, 10, True, 40)
        references[p] = {"reference": ref, "check": check, "uncertainty": check["grid_error"],
                         "native_endpoint_difference": 0.0}
    rows = []
    for p in cg.PROBLEMS:
        for seed in cg.SEEDS:
            for q in range(cg.FIRST_QUARTER, cg.LAST_QUARTER[p] + 1):
                rtol = cg.quarter_decade(q)
                for arm in cg.ARMS:
                    err, att, rej, ok = model.cell(p, seed, q, arm)
                    row = run_row(p, arm, rtol, seed, err, att, rej, ok, model.clipped)
                    row["first_difference_from_PREDcap"] = None
                    rows.append(row)
    doc = dict(cg.HEADER)
    doc["references"] = references
    doc["rows"] = rows
    return doc


class CheckerCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="ct01-check-"))

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def check(self, doc=None, text=None, native=NATIVE, repo_root=ROOT):
        runs = self.tmp / "RUNS.json"
        runs.write_text(text if text is not None else json.dumps(doc))
        out = self.tmp / "RESULTS.json"
        if out.exists():
            out.unlink()
        code = cg.run(runs, native, out, repo_root)
        return code, json.loads(out.read_text())

    def assert_invalid(self, doc=None, needle="", **kw):
        code, res = self.check(doc, **kw)
        self.assertEqual(code, 2)
        self.assertEqual(res["verdict"], "INVALID")
        self.assertTrue(any(needle in r for r in res["reasons"]), res["reasons"][:5])

    def assert_fail(self, doc, item):
        code, res = self.check(doc)
        self.assertEqual(code, 1, {k: g["pass"] for k, g in res.get("gates", {}).items()})
        self.assertEqual(res["verdict"], "FAIL")
        failing = [k for k, g in res["gates"].items() if not g["pass"]]
        self.assertEqual(failing, [item])
        return res


class TestValid(CheckerCase):
    def test_valid_fixture_passes(self):
        code, res = self.check(make_doc(Model()))
        self.assertEqual(code, 0, {k: g["pass"] for k, g in res["gates"].items()})
        self.assertEqual(res["verdict"], "PASS")
        self.assertEqual(res["validity"]["reference_limited_count"], 0)
        g = res["gates"]
        self.assertGreaterEqual(len(g["1_matched_error_work"]["van_der_pol"]["passing_E"]), 3)
        self.assertAlmostEqual(g["3_rejections"]["ratio"], 0.25, places=2)
        self.assertEqual(res["reported"]["PREDcap2_vs_PREDcap"]["differing_cells"], 0)
        self.assertEqual(res["reported"]["telemetry_totals"]["I"]["all"]["runs"], 644)

    def test_cli_exit_code_and_immutable_output(self):
        runs = self.tmp / "RUNS.json"
        runs.write_text(json.dumps(make_doc(Model())))
        out = self.tmp / "RESULTS.json"
        cmd = [sys.executable, str(TOOLS / "controller_grid_check.py"), "--runs", str(runs), "--native", str(NATIVE),
               "--output", str(out)]
        self.assertEqual(subprocess.run(cmd, capture_output=True).returncode, 0)
        again = subprocess.run(cmd, capture_output=True, text=True)
        self.assertNotEqual(again.returncode, 0)
        self.assertIn("immutable output exists", again.stderr)


class TestInvalid(CheckerCase):
    def test_strict_json(self):
        text = json.dumps(make_doc(Model()))
        self.assert_invalid(text=text.replace('"grid_error": 0.0', '"grid_error": NaN', 1), needle="strict JSON")

    def test_duplicate_key(self):
        text = json.dumps(make_doc(Model()))
        self.assert_invalid(text=text.replace('"ok": true,', '"ok": true, "ok": true,', 1), needle="duplicate")

    def test_native_pin(self):
        bad = self.tmp / "NATIVE.json"
        bad.write_bytes(NATIVE.read_bytes() + b"\n")
        self.assert_invalid(make_doc(Model()), needle="registered immutable input", native=bad)

    def test_bad_hex(self):
        doc = make_doc(Model())
        doc["rows"][5]["grid_states"] = [list(s) for s in doc["rows"][5]["grid_states"]]
        doc["rows"][5]["grid_states"][3][0] = "7ff8000000000000"
        self.assert_invalid(doc, needle="non-finite")

    def test_missing_row(self):
        doc = make_doc(Model())
        del doc["rows"][100]
        self.assert_invalid(doc, needle="[row-set]")

    def test_duplicated_row(self):
        doc = make_doc(Model())
        doc["rows"][101] = dict(doc["rows"][100])
        self.assert_invalid(doc, needle="duplicate raw key")

    def test_error_mismatch(self):
        doc = make_doc(Model())
        doc["rows"][7]["grid_error"] = math.nextafter(doc["rows"][7]["grid_error"], 1.0)
        self.assert_invalid(doc, needle="[error-recomputation]")

    def test_endpoint_error_mismatch(self):
        doc = make_doc(Model())
        doc["rows"][8]["endpoint_error"] = doc["rows"][8]["endpoint_error"] * 2.0
        self.assert_invalid(doc, needle="[error-recomputation]")

    def test_parity_test_missing(self):
        self.assert_invalid(make_doc(Model()), needle="[parity-test]", repo_root=self.tmp)

    def test_grid_inactive(self):
        doc = make_doc(Model())
        doc["rows"][9]["output_clipped_steps"] = 0
        doc["rows"][9]["telemetry"] = dict(doc["rows"][9]["telemetry"], clipped_landings=0)
        self.assert_invalid(doc, needle="[grid-inactive]")

    def test_reference_limited_above_five_percent(self):
        model = Model()
        model.check_e = 1.0e-9
        self.assert_invalid(make_doc(model), needle="[reference-limited]")

    def test_telemetry_unbound(self):
        doc = make_doc(Model())
        doc["rows"][11]["telemetry"] = dict(doc["rows"][11]["telemetry"], updates=1)
        self.assert_invalid(doc, needle="telemetry.updates")


class TestFail(CheckerCase):
    def test_item1_matched_error_work(self):
        model = Model()
        model.vdp_work = 1.0
        self.assert_fail(make_doc(model), "1_matched_error_work")

    def test_item2_worst_seed_guard(self):
        model = Model()
        model.work_override[("hires", 7.0e-5)] = 1.3
        res = self.assert_fail(make_doc(model), "2_worst_seed_guard")
        self.assertLessEqual(res["gates"]["1_matched_error_work"]["others"]["hires"]["worst_R"], 1.06)

    def test_item3_rejections(self):
        model = Model()
        model.vdp_rejection = 0.2
        self.assert_fail(make_doc(model), "3_rejections")

    def test_item4_relative_calibration(self):
        model = Model()
        model.error_factor["robertson"] = 3.0
        # The same frontier (3x the error at 3^-0.2 the work): only the calibration moves.
        for seed in cg.SEEDS:
            model.work_override[("robertson", seed)] = 3.0 ** -0.2
        res = self.assert_fail(make_doc(model), "4_relative_calibration")
        rob = res["gates"]["4_relative_calibration"]["problems"]["robertson"]
        self.assertFalse(rob["a_pass"])
        self.assertFalse(rob["b_pass"])

    def test_item5_catastrophic_cell(self):
        model = Model()
        model.cell_error[("brusselator-1d-50", 2.0e-4, 28)] = 20.0
        res = self.assert_fail(make_doc(model), "5_catastrophic_cell_guard")
        self.assertEqual(len(res["gates"]["5_catastrophic_cell_guard"]["catastrophic_cells"]), 1)
        self.assertEqual(len(res["reported"]["cells_err_over_rtol_above_10"]), 1)

    def test_item5_failing_cell(self):
        model = Model()
        model.failing.add(("hires", 2.0e-5, 20))
        res = self.assert_fail(make_doc(model), "5_catastrophic_cell_guard")
        self.assertEqual(len(res["gates"]["5_catastrophic_cell_guard"]["failing_cells"]), 1)

    def test_anchor_seed_is_never_gated(self):
        model = Model()
        model.cell_error[("hires", cg.ANCHOR_SEED, 30)] = 65.0
        model.work_override[("hires", cg.ANCHOR_SEED)] = 2.0
        code, res = self.check(make_doc(model))
        self.assertEqual(code, 0)
        anchor = res["reported"]["anchor_seed"]["cells_above_10"]
        self.assertEqual(len(anchor), 1)
        self.assertEqual(set(anchor[0]["neighbouring_seeds"]), {str(2.0e-6), str(7.0e-6)})


if __name__ == "__main__":
    unittest.main()
