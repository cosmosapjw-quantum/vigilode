#!/usr/bin/env python3
"""Tests of tools/triage/triage_check.py (component triage decision engine) on synthetic fixtures.

Every fixture is written to a temporary directory; no committed file is read except the tools themselves.
Problems are dimension-d copies of a constant reference y(t) = 1 on the endpoint grid with atol_scale 1, so a
row whose final state is 1 + 2e has E = e exactly in the CORPUS weights 1 / (1 + |ref|).

Run: python3 -m pytest tools/test_triage_check.py -q   (or python3 -m unittest tools/test_triage_check.py)
"""

from __future__ import annotations

import copy
import json
import struct
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
TRIAGE = ROOT / "tools" / "triage"
for _p in (str(ROOT / "tools"), str(TRIAGE)):
    if _p not in sys.path:
        sys.path.insert(0, _p)
import triage_check as tch  # noqa: E402

CAMPAIGN = "ct-test"
# Test-only copy of the THRESHOLDS structure with a short target list (the frozen file is not read here).
THRESHOLDS = {
    "schema": "vigilode-triage-thresholds-v1",
    "frozen_with": "test fixture",
    "accuracy": {"achieved_error_metric": "x", "requested_error_metric": "x", "targets": [1e-3, 1e-5],
                 "reference_limited_factor": 0.1, "h0_scales_required": [0.5, 1.0, 2.0]},
    "reliability_c2": {"cell_fraction": 0.95, "ratio": 10.0, "ratio_max": 100.0},
    "frontier": {"gain_max": 0.98, "neutral_max": 1.02, "promote_cell_fraction": 0.90, "promote_max_ratio": 1.10,
                 "primary_cost": "ir_warm",
                 "secondary_costs": ["ir_cold", "allocations_warm", "allocated_bytes_warm", "rhs_evaluations",
                                     "jvp_vectors", "direct_factorizations", "linear_iterations"]},
    "allocation": {"warm_window_allocations_max": 0, "allow_documented_constant_independent_of_n": True},
    "specialize": {"min_positives": 5, "threshold_rank_shift": 1,
                   "features": ["dimension", "structure", "jacobian", "mass", "oscillatory", "exact_solution",
                                "grid", "rtol"]},
    "optimize": {"headroom_ratio_max": 1.02, "external_counted_work_gap": 1.5},
    "timing": {"authority": "diagnostic", "conflict_threshold": 0.15},
    "parity": {"max_relative_error": 1e-13},
    "external": {"peers_field": "external_peers", "admissibility": "single row",
                 "counted_work": ["rhs_evaluations", "jacobian_evaluations", "lu_factorizations"]},
}
RTOLS = [1e-4, 1e-6]
H0 = [0.5, 1.0, 2.0]


def hx(value: float) -> str:
    return struct.pack(">d", value).hex()


class Fixture:
    """Builder of COMPONENTS/CORPUS/REFERENCES/RUNS/PROFILE/ALLOCATIONS for one campaign."""

    def __init__(self):
        self.thresholds = copy.deepcopy(THRESHOLDS)
        self.components: dict[str, dict] = {}
        self.problems: dict[str, dict] = {}
        self.refs: list[dict] = []
        self.rows: list[dict] = []
        self.profile: list[dict] = []
        self.alloc: list[dict] = []
        self.probes: list[dict] | None = None  # None: one passing must-refuse probe per component
        self.timing: dict | None = None
        self.attribution: dict | None = None

    def component(self, cid, slot, loc=100, family="F1", **kw):
        spec = {"id": cid, "family": family, "slot": slot, "files": [], "reach": "D", "selector": "s",
                "capability": "c", "tiers": ["G", "P", "C", "V"], "arms": [], "prior": [], "blocked_by": [],
                "loc": loc, "static_class": None, "runtime_dependents": [], "unique_capability": False, "notes": ""}
        spec.update(kw)
        self.components[cid] = spec
        return spec

    def problem(self, pid, dimension=1, tier="G", partition="calibration", rho=0.0, **features):
        spec = {"id": pid, "tier": tier, "partition": partition, "dimension": dimension, "structure": "dense",
                "jacobian": "analytic", "mass": False, "exact_solution": False, "oscillatory": False,
                "t_span": [0.0, 1.0], "atol_scale": 1.0, "reference": {"kind": "two_method", "methods": ["Radau", "LSODA"]}}
        spec.update(features)
        self.problems[pid] = spec
        self.refs.append({"problem": pid, "grid": "endpoint", "kind": "two_method", "status": "ok", "reason": None,
                          "output_times": [0.0, 1.0], "states_hex": [[hx(1.0)] * dimension] * 2,
                          "rho_hex": [[hx(0.0)] * dimension, [hx(rho)] * dimension], "rho_rule": "x",
                          "methods": [], "canonical_method": "Radau", "model_source": "test"})
        return spec

    def row(self, arm, comps, pid, rtol, h0, err, status="ok", deterministic=True, counters=None):
        dim = self.problems[pid]["dimension"]
        ok = status == "ok"
        row = {"schema": "vigilode-triage-row-v1", "campaign": CAMPAIGN, "arm": arm, "components": list(comps),
               "problem": pid, "tier": self.problems[pid]["tier"], "dimension": dim, "rtol": rtol, "atol": rtol,
               "grid": "endpoint", "h0_scale": h0, "h0": h0 * 1e-6, "status": status,
               "error": None if ok else "failed", "success": ok, "attempts": 10, "accepted_steps": 9,
               "rejected_steps": 1, "output_times": [0.0, 1.0] if ok else [],
               "states_hex": [[hx(1.0)] * dim, [hx(1.0 + 2.0 * err)] * dim] if ok else [],
               "counters": counters if counters is not None else {"rhs_evaluations": 100, "jacobian_builds": 5,
                                                                  "jvp_vectors": 0, "direct_factorizations": 10,
                                                                  "linear_iterations": 0},
               "extra_work": None, "counter_semantics": "work-counters-v2-attempted", "deterministic": deterministic,
               "repetitions": 1, "binary": {"sha256": "ab" * 32, "profile": "bench"},
               "source": {"commit": "0" * 40, "tree_clean": True}}
        if status == "process_abort":
            row["process"] = {"kind": "abort", "exit_status": -6, "signal": "SIGABRT", "stderr_tail": "",
                              "stdout_tail": ""}
            row["counters"] = None
            row["deterministic"] = None
        self.rows.append(row)
        return row

    def arm(self, arm, comps, pid, ir=1000, err_scale=1.0, alloc=0, profile=True, rtols=RTOLS, h0s=H0):
        """Rows for every rtol x h0 (E = err_scale * rtol), a PROFILE row and an ALLOCATIONS row per rtol."""
        for rtol in rtols:
            for h0 in h0s:
                self.row(arm, comps, pid, rtol, h0, err_scale * rtol)
            if profile:
                self.profile_row(arm, pid, rtol, ir)
            self.alloc_row(arm, pid, rtol, alloc)

    def profile_row(self, arm, pid, rtol, ir, deterministic=True):
        self.profile.append({"schema": "vigilode-triage-profile-v1", "campaign": CAMPAIGN, "arm": arm, "problem": pid,
                             "rtol": rtol, "grid": "endpoint", "h0_scale": 1.0, "status": "ok", "reason": None,
                             "ir": {"0": 100, "1": 100 + 2 * ir, "2": 100 + 3 * ir, "1b": 100 + 2 * ir},
                             "ir_warm": ir, "ir_cold": 2 * ir, "deterministic": deterministic,
                             "summary": {"success": True, "attempts": 10}, "cachegrind": None, "valgrind": "v",
                             "binary": {"sha256": "ab" * 32, "profile": "bench"},
                             "source": {"commit": "0" * 40, "tree_clean": True}})

    def alloc_row(self, arm, pid, rtol, allocations):
        window = {"allocations": allocations, "deallocations": allocations, "allocated_bytes": 8 * allocations,
                  "peak_live_bytes": 0}
        self.alloc.append({"schema": "vigilode-triage-allocations-v1", "campaign": CAMPAIGN, "arm": arm,
                           "problem": pid, "rtol": rtol, "grid": "endpoint", "h0_scale": 1.0, "warmups": 2,
                           "windows_requested": 1, "status": "ok", "reason": None, "cold": window,
                           "windows": [window], "warm": window, "warm_rule": "max", "process": None,
                           "binary": {"sha256": "ab" * 32}, "source": {"commit": "0" * 40, "tree_clean": True}})

    def probe(self, cid: str, status: str = "refused", must_refuse: bool = True) -> dict:
        """A PROBES.jsonl row (tools/triage/README.md): a row of the row schema plus `probe`."""
        ok = status == "ok"
        return {"schema": "vigilode-triage-row-v1", "campaign": CAMPAIGN, "arm": "probe-arm", "components": [cid],
                "problem": "probe-nan-y0", "tier": "V", "dimension": 1, "rtol": 1e-6, "atol": 1e-6,
                "grid": "endpoint", "h0_scale": 1.0, "h0": 1e-6, "status": status,
                "error": None if ok else "typed refusal", "success": ok, "attempts": 0, "accepted_steps": 0,
                "rejected_steps": 0, "output_times": [], "states_hex": [], "counters": None, "extra_work": None,
                "counter_semantics": "work-counters-v2-attempted", "deterministic": True, "repetitions": 1,
                "binary": {"sha256": "ab" * 32, "profile": "bench"}, "source": {"commit": "0" * 40, "tree_clean": True},
                "probe": {"id": f"nan-y0-{cid}", "must_refuse": must_refuse, "components": [cid]}}

    def write(self, directory: Path) -> list[str]:
        d = Path(directory)
        d.mkdir(parents=True, exist_ok=True)
        corpus = {"schema": "vigilode-triage-corpus-v1", "problems": list(self.problems.values()),
                  "sweep": {"rtols": RTOLS, "grids": ["endpoint"], "h0_scales": H0},
                  "profile_subset": {"rtols": RTOLS, "max_dimension": 10000}}
        comps = {"schema": "vigilode-triage-components-v1", "base_commit": "0" * 40,
                 "components": list(self.components.values())}
        refs = {"schema": "vigilode-triage-references-v1", "references": self.refs}
        (d / "THRESHOLDS.json").write_text(json.dumps(self.thresholds))
        (d / "CORPUS.json").write_text(json.dumps(corpus))
        (d / "COMPONENTS.json").write_text(json.dumps(comps))
        (d / "REFERENCES.json").write_text(json.dumps(refs))
        for name, rows in (("RUNS.jsonl", self.rows), ("PROFILE.jsonl", self.profile),
                           ("ALLOCATIONS.jsonl", self.alloc)):
            (d / name).write_text("".join(json.dumps(r) + "\n" for r in rows))
        argv = ["--campaign", CAMPAIGN, "--runs", str(d / "RUNS.jsonl"), "--references", str(d / "REFERENCES.json"),
                "--profile", str(d / "PROFILE.jsonl"), "--allocations", str(d / "ALLOCATIONS.jsonl"),
                "--components", str(d / "COMPONENTS.json"), "--corpus", str(d / "CORPUS.json"),
                "--thresholds", str(d / "THRESHOLDS.json"), "--results", str(d / "RESULTS.json"),
                "--classification", str(d / "CLASSIFICATION.json")]
        probes = self.probes if self.probes is not None else [self.probe(cid) for cid in sorted(self.components)]
        (d / "PROBES.jsonl").write_text("".join(json.dumps(r) + "\n" for r in probes))
        argv += ["--probes", str(d / "PROBES.jsonl")]
        if self.timing is not None:
            (d / "TIMING.json").write_text(json.dumps(self.timing))
            argv += ["--timing", str(d / "TIMING.json")]
        if self.attribution is not None:
            (d / "ATTRIBUTION.json").write_text(json.dumps(self.attribution))
            argv += ["--attribution", str(d / "ATTRIBUTION.json")]
        return argv

    def check(self, directory: Path) -> tuple[dict, dict, list[str]]:
        argv = self.write(directory)
        code = tch.main(argv)
        if code != 0:
            raise AssertionError(f"triage_check exit {code}")
        d = Path(directory)
        return (json.loads((d / "RESULTS.json").read_text()), json.loads((d / "CLASSIFICATION.json").read_text()),
                argv)


def pair_fixture(cost_a=1000, cost_b=2000, problems=("P1", "P2"), loc_a=100, loc_b=100) -> Fixture:
    """Two drivers of one slot sharing la.x: A = {drv.a, la.x}, B = {drv.b, la.x}."""
    fx = Fixture()
    fx.component("drv.a", "drv", loc=loc_a)
    fx.component("drv.b", "drv", loc=loc_b)
    fx.component("la.x", "la")
    for pid in problems:
        fx.problem(pid)
        fx.arm("A", ["drv.a", "la.x"], pid, ir=cost_a)
        fx.arm("B", ["drv.b", "la.x"], pid, ir=cost_b)
    return fx


def rules(entry: dict) -> list[str]:
    return [step["rule"] for step in entry["rule_path"]]


def component(classification: dict, cid: str) -> dict:
    return next(c for c in classification["components"] if c["id"] == cid)


def comparisons(results: dict, cid: str, **match) -> list[dict]:
    return [r for r in results["comparisons"] if r["component"] == cid
            and all(r[k] == v for k, v in match.items())]


def group(results: dict, arm: str, problem: str, rtol: float) -> dict:
    return next(g for g in results["groups"] if (g["arm"], g["problem"], g["rtol"]) == (arm, problem, rtol))


class TriageCheckTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="triage_check_")
        self.tmp = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def test_eps_violating_row_cannot_make_cell_admissible(self):
        fx = pair_fixture()
        # A at rtol 1e-6 reaches E = 2e-5 > 1e-5 on P1 at h0 = 1 only: no A group is admissible at 1e-5.
        for row in fx.rows:
            if row["arm"] == "A" and row["problem"] == "P1" and row["rtol"] == 1e-6 and row["h0_scale"] == 1.0:
                row["states_hex"][1] = [hx(1.0 + 2.0 * 2e-5)]
        results, classification, _ = fx.check(self.tmp)
        g = group(results, "A", "P1", 1e-6)
        states = {t["target"]: t for t in g["targets"]}
        self.assertEqual(states[1e-3]["state"], "admissible")
        self.assertEqual(states[1e-5]["state"], "inadmissible")
        self.assertIn("h0x1.0: E_up 2.00000000000", states[1e-5]["reason"])
        self.assertIn("> 1e-05", states[1e-5]["reason"])
        loss = comparisons(results, "drv.a", problem="P1", target=1e-5)
        self.assertEqual([r["outcome"] for r in loss], ["capability_loss"])
        gain = comparisons(results, "drv.b", problem="P1", target=1e-5)
        self.assertEqual([r["outcome"] for r in gain], ["capability_gain"])

    def test_missing_profile_gives_null_cost_with_reason(self):
        fx = pair_fixture()
        fx.profile = [p for p in fx.profile if not (p["arm"] == "A" and p["problem"] == "P1" and p["rtol"] == 1e-4)]
        results, _, _ = fx.check(self.tmp)
        g = group(results, "A", "P1", 1e-4)
        self.assertIsNone(g["cost"])
        self.assertEqual(g["cost_reason"], "profile_missing")
        # At 1e-3 both rtol groups of A are admissible; the missing one makes the configuration cost incomplete.
        rec = comparisons(results, "drv.a", problem="P1", target=1e-3)[0]
        self.assertEqual(rec["outcome"], "excluded")
        self.assertIsNone(rec["ratio"])
        self.assertIn("insufficient_evidence: cost", rec["reason"])
        self.assertIn("profile_missing", rec["reason"])

    def test_one_h0_failure_blocks_admissibility(self):
        fx = pair_fixture()
        for row in fx.rows:
            if row["arm"] == "A" and row["problem"] == "P2" and row["rtol"] == 1e-4 and row["h0_scale"] == 2.0:
                row.update(status="solver_failure", success=False, output_times=[], states_hex=[], error="underflow")
        results, _, _ = fx.check(self.tmp)
        g = group(results, "A", "P2", 1e-4)
        for target in g["targets"]:
            self.assertEqual(target["state"], "inadmissible")
            self.assertIn("h0x2.0: status solver_failure", target["reason"])
        # The rtol 1e-6 group still makes A admissible at 1e-3, so the comparison uses it.
        rec = comparisons(results, "drv.a", problem="P2", target=1e-3)[0]
        self.assertEqual(rec["rtol"], 1e-6)

    def test_dominated_arm_on_all_cells_is_pruned(self):
        fx = pair_fixture(cost_a=1000, cost_b=2000)
        results, classification, _ = fx.check(self.tmp)
        b = component(classification, "drv.b")
        self.assertEqual(b["label"], "PRUNE")
        self.assertEqual(b["evidence"]["outcomes"]["LOSS"], b["evidence"]["comparisons"])
        a = component(classification, "drv.a")
        self.assertEqual(a["label"], "PROMOTE")
        self.assertEqual({r["ratio"] for r in comparisons(results, "drv.a")}, {0.5})

    def test_attribution_and_strongest_rival_selection(self):
        fx = Fixture()
        for cid, slot in (("drv.a", "drv"), ("drv.b", "drv"), ("drv.c", "drv"), ("la.x", "la"), ("la.y", "la")):
            fx.component(cid, slot)
        fx.problem("P1")
        fx.arm("A", ["drv.a", "la.x"], "P1", ir=1000)
        fx.arm("A2", ["drv.b", "la.x"], "P1", ir=500)   # attribution counterpart of A for drv.a
        fx.arm("C", ["drv.a", "la.y"], "P1", ir=1000)   # no {drv.b|drv.c, la.y} arm: strongest for drv.a in C
        fx.arm("D", ["drv.c", "la.x"], "P1", ir=2000)   # second attribution counterpart of A
        results, _, _ = fx.check(self.tmp)
        rec_a = comparisons(results, "drv.a", arm="A", target=1e-3)
        self.assertEqual(sorted((r["kind"], r["counterpart"]) for r in rec_a),
                         [("attribution", "A2"), ("attribution", "D")])
        rec_c = comparisons(results, "drv.a", arm="C", target=1e-3)
        self.assertEqual([(r["kind"], r["counterpart"]) for r in rec_c], [("strongest", "A2")])  # cheapest rival
        self.assertEqual(rec_c[0]["ratio"], 2.0)
        self.assertEqual(rec_c[0]["outcome"], "LOSS")

    def test_capability_loss_and_gain(self):
        fx = pair_fixture(problems=("P1",))
        fx.problem("P2")
        fx.arm("A", ["drv.a", "la.x"], "P2", err_scale=500.0)
        fx.arm("B", ["drv.b", "la.x"], "P2")
        results, classification, _ = fx.check(self.tmp)
        # A: rtol 1e-4 -> E 0.05, rtol 1e-6 -> E 5e-4 (admissible at 1e-3 only); at 1e-5 A is inadmissible.
        self.assertEqual(comparisons(results, "drv.a", problem="P2", target=1e-5)[0]["outcome"], "capability_loss")
        self.assertEqual(comparisons(results, "drv.b", problem="P2", target=1e-5)[0]["outcome"], "capability_gain")
        self.assertEqual(comparisons(results, "drv.a", problem="P2", target=1e-3)[0]["rtol"], 1e-6)
        counts = component(classification, "drv.a")["evidence"]["outcomes"]
        self.assertEqual(counts["capability_loss"], 1)

    def test_reference_limited_cells_are_excluded(self):
        fx = pair_fixture(problems=("P1",))
        fx.problem("P3", rho=2e-5)  # E_rho = 1e-5: > 0.1 * 1e-5 but < 0.1 * 1e-3
        fx.arm("A", ["drv.a", "la.x"], "P3", ir=1000, err_scale=0.01)
        fx.arm("B", ["drv.b", "la.x"], "P3", ir=2000, err_scale=0.01)
        results, _, _ = fx.check(self.tmp)
        ref = next(r for r in results["references"] if r["problem"] == "P3")
        self.assertEqual(ref["reference_limited_targets"], [1e-5])
        rec = comparisons(results, "drv.a", problem="P3", target=1e-5)[0]
        self.assertEqual(rec["outcome"], "excluded")
        self.assertTrue(rec["reason"].startswith("reference_limited"))
        self.assertEqual(comparisons(results, "drv.a", problem="P3", target=1e-3)[0]["outcome"], "GAIN")

    def test_provisional_promote_falls_to_optimize_not_prune(self):
        fx = pair_fixture()
        fx.components["drv.a"]["blocked_by"] = ["la.x"]
        _, classification, _ = fx.check(self.tmp)
        a = component(classification, "drv.a")
        self.assertTrue(a["provisional"])
        self.assertEqual(a["label"], "OPTIMIZE")
        self.assertEqual(a["criteria"]["promote"]["status"], "pass")
        self.assertIn("PROMOTE_BLOCKED_PROVISIONAL", a["flags"])
        step = next(s for s in a["rule_path"] if s["rule"] == "PROMOTE_BLOCKED_PROVISIONAL")
        self.assertEqual(step["kind"], "contract")
        self.assertIn("blocked_by ['la.x']", step["detail"])
        self.assertEqual(rules(a)[0], "BLOCKED_BY")
        self.assertEqual(a["blocked_by"], ["la.x"])

    def test_c3_missing_blocks_promote_and_is_provisional(self):
        fx = pair_fixture()
        fx.probes = [fx.probe("drv.b"), fx.probe("la.x")]  # nothing covers drv.a
        _, classification, _ = fx.check(self.tmp)
        a = component(classification, "drv.a")
        self.assertIsNone(a["criteria"]["c3"]["status"])
        self.assertEqual((a["label"], a["provisional"]), ("OPTIMIZE", True))
        self.assertEqual(sorted(a["flags"]), ["C3_MISSING", "PROMOTE_BLOCKED_PROVISIONAL"])
        step = next(s for s in a["rule_path"] if s["rule"] == "PROMOTE_BLOCKED_PROVISIONAL")
        self.assertIn("C3 missing", step["detail"])
        self.assertEqual(component(classification, "la.x")["label"], "PROMOTE")

    def test_no_evidence_labels(self):
        fx = pair_fixture()
        fx.component("drv.ghost", "drv", unique_capability=False)
        fx.component("drv.unique", "drv.unique", unique_capability=True)
        fx.component("drv.relocate", "drv", static_class="PRUNE_RELOCATE")
        _, classification, _ = fx.check(self.tmp)
        ghost, unique = component(classification, "drv.ghost"), component(classification, "drv.unique")
        self.assertEqual((ghost["label"], ghost["provisional"]), ("PRUNE", True))
        self.assertEqual((unique["label"], unique["provisional"]), ("OPTIMIZE", True))
        for e in (ghost, unique):
            self.assertIn("NO_EVIDENCE", e["flags"])
            self.assertEqual(rules(e)[-1], "NO_EVIDENCE")
            self.assertTrue(any("insufficient_evidence" in s["detail"] for s in e["rule_path"]
                                if s["rule"] == "NO_EVIDENCE"))
        relocate = component(classification, "drv.relocate")
        self.assertEqual((relocate["label"], relocate["prune_kind"], relocate["provisional"]),
                         ("PRUNE", "relocate", False))
        self.assertEqual(relocate["flags"], [])

    def specialize_fixture(self, holdout_small_loses: bool) -> Fixture:
        fx = Fixture()
        fx.component("drv.a", "drv")
        fx.component("drv.b", "drv")
        fx.component("la.x", "la")
        layout = [("C2", 2, "calibration", 2000), ("C4", 4, "calibration", 2000),
                  ("C100", 100, "calibration", 500), ("C200", 200, "calibration", 500),
                  ("H1", 1, "holdout", 500 if holdout_small_loses else 2000),
                  ("H3", 3, "holdout", 500 if holdout_small_loses else 2000), ("H5", 5, "holdout", 1000)]
        layout += [(f"H{d}", d, "holdout", 500) for d in (60, 70, 80, 90, 110)]
        for pid, dim, partition, cost_b in layout:
            fx.problem(pid, dimension=dim, partition=partition)
            fx.arm("A", ["drv.a", "la.x"], pid, ir=1000)
            fx.arm("B", ["drv.b", "la.x"], pid, ir=cost_b)
        return fx

    def test_specialize_predicate_found_and_validated_on_holdout(self):
        results, classification, _ = self.specialize_fixture(False).check(self.tmp)
        a = component(classification, "drv.a")
        self.assertEqual(a["label"], "SPECIALIZE", a["rule_path"])
        pred = a["predicate"]
        self.assertEqual((pred["feature"], pred["op"], pred["threshold"]), ("dimension", "le", 4))
        self.assertEqual(pred["calibration"]["wins_covered"], 4)
        self.assertEqual(pred["holdout"]["verdict"], "PASS")
        self.assertEqual(pred["holdout"]["n_positives"], 10)
        self.assertEqual(pred["holdout"]["holdout_wins_recommended"], 4)
        self.assertTrue(pred["holdout"]["rank_stable"])

    def test_provisional_specialize_stays_specialize(self):
        fx = self.specialize_fixture(False)
        fx.components["drv.a"]["blocked_by"] = ["la.x"]
        _, classification, _ = fx.check(self.tmp)
        a = component(classification, "drv.a")
        self.assertEqual((a["label"], a["provisional"]), ("SPECIALIZE", True))
        self.assertEqual(rules(a), ["BLOCKED_BY", "CORRECTNESS_OK", "NOT_PROMOTE", "SPECIALIZE"])
        self.assertIsNotNone(a["predicate"])

    def test_specialize_rejected_when_holdout_disagrees(self):
        results, classification, _ = self.specialize_fixture(True).check(self.tmp / "b")
        a = component(classification, "drv.a")
        self.assertNotEqual(a["label"], "SPECIALIZE")
        self.assertIsNone(a["predicate"])
        crit = a["criteria"]["specialize"]
        self.assertEqual(crit["holdout_verdict"], "FAIL")  # stable: losses at dimension 1 and 3 in the holdout
        self.assertEqual(crit["candidate_predicate"]["threshold"], 4)
        self.assertEqual(crit["holdout"]["n_positives"], 14)
        self.assertEqual(a["label"], "PRUNE")

    def test_timing_conflict_flag(self):
        fx = pair_fixture()
        fx.timing = {"schema": "vigilode-triage-timing-v1", "host": {"wall_clock_admissible": True},
                     "entries": [{"arm": "A", "counterpart": "B", "problem": "P1", "grid": "endpoint", "rtol": 1e-4,
                                  "wall_ratio": 1.3, "interval": [1.2, 1.4]},
                                 {"arm": "B", "counterpart": "A", "problem": "P2", "grid": "endpoint", "rtol": 1e-4,
                                  "wall_ratio": 1.9, "interval": [1.8, 2.0]}]}
        results, classification, _ = fx.check(self.tmp)
        a = component(classification, "drv.a")
        self.assertIn("TIMING_CONFLICT", a["flags"])
        self.assertTrue(a["provisional"])
        self.assertNotEqual(a["label"], "PROMOTE")
        b = component(classification, "drv.b")
        self.assertNotIn("TIMING_CONFLICT", b["flags"])  # B is slower in Ir and in wall time: no conflict
        entries = results["timing"]["entries"]
        self.assertEqual([e["conflict"] for e in entries], [True, False])
        self.assertEqual(entries[0]["ir_ratio"], 0.5)

    def test_static_prune_relocate(self):
        fx = pair_fixture()
        fx.components["drv.a"]["static_class"] = "PRUNE_RELOCATE"
        _, classification, _ = fx.check(self.tmp)
        a = component(classification, "drv.a")
        self.assertEqual((a["label"], a["prune_kind"]), ("PRUNE", "relocate"))
        self.assertEqual(a["rule_path"][0], {"rule": "STATIC_PRUNE_RELOCATE", "kind": "static",
                                             "detail": "static_class PRUNE_RELOCATE -> PRUNE(relocate)"})

    def test_neutral_tiebreak_by_loc(self):
        fx = pair_fixture(cost_a=1000, cost_b=1010, loc_a=500, loc_b=100)
        results, classification, _ = fx.check(self.tmp)
        self.assertEqual({r["outcome"] for r in comparisons(results, "drv.a")}, {"NEUTRAL"})
        a, b = component(classification, "drv.a"), component(classification, "drv.b")
        self.assertEqual(b["label"], "PROMOTE")
        self.assertEqual(a["label"], "PRUNE")
        self.assertEqual((a["rule_path"][-1]["rule"], a["rule_path"][-1]["kind"]), ("NEUTRAL_TIEBREAK_LOC", "tiebreak"))
        self.assertEqual(rules(a)[-2], "PROMOTE")  # it met PROMOTE, then lost the tie-break

    def test_c1_process_abort_blames_unshared_component(self):
        fx = pair_fixture()
        for row in fx.rows:
            if row["arm"] == "A" and row["problem"] == "P1" and row["rtol"] == 1e-4 and row["h0_scale"] == 0.5:
                row.update(status="process_abort", success=False, output_times=[], states_hex=[], error="SIGABRT",
                           counters=None, deterministic=None,
                           process={"kind": "abort", "exit_status": -6, "signal": "SIGABRT", "stderr_tail": "",
                                    "stdout_tail": ""})
        _, classification, _ = fx.check(self.tmp)
        a, x = component(classification, "drv.a"), component(classification, "la.x")
        self.assertEqual(a["criteria"]["c1"]["status"], "fail")
        self.assertEqual(a["label"], "PRUNE")
        self.assertEqual(x["criteria"]["c1"]["status"], "pass")  # exonerated: B ran the same cell cleanly

    def test_c1_failure_with_repair_path(self):
        def aborted(fx):
            for row in fx.rows:
                if row["arm"] == "A" and row["problem"] == "P1" and row["rtol"] == 1e-4 and row["h0_scale"] == 0.5:
                    row.update(status="process_abort", success=False, output_times=[], states_hex=[], error="abort",
                               counters=None, deterministic=None,
                               process={"kind": "abort", "exit_status": -6, "signal": "SIGABRT", "stderr_tail": "",
                                        "stdout_tail": ""})
        fx = pair_fixture()
        fx.components["drv.a"].update(repair_path="guard the stage residual before the LU (issue X)",
                                      unique_capability=True)
        aborted(fx)
        _, classification, _ = fx.check(self.tmp / "repairable")
        a = component(classification, "drv.a")
        self.assertEqual((a["label"], a["provisional"]), ("OPTIMIZE", True))
        self.assertEqual(rules(a)[-2:], ["C1_FAIL", "C_FAIL_REPAIRABLE"])
        self.assertEqual(a["rule_path"][-1], {"rule": "C_FAIL_REPAIRABLE", "kind": "contract",
                                              "detail": "guard the stage residual before the LU (issue X)"})
        # repair_path without unique_capability, or a unique capability without repair_path: PRUNE as before
        for name, update in (("not_unique", {"repair_path": "fix", "unique_capability": False}),
                             ("no_repair", {"repair_path": None, "unique_capability": True})):
            fx = pair_fixture()
            fx.components["drv.a"].update(update)
            aborted(fx)
            _, classification, _ = fx.check(self.tmp / name)
            a = component(classification, "drv.a")
            self.assertEqual((a["label"], a["provisional"], rules(a)[-1]), ("PRUNE", False, "C1_FAIL"), name)

    def test_c3_probe_must_refuse(self):
        fx = pair_fixture()
        probe_ok = copy.deepcopy(fx.rows[0])
        probe_ok.update(status="refused", success=False, output_times=[], states_hex=[],
                        probe={"id": "nan-y0", "must_refuse": True, "components": ["drv.a"]})
        probe_bad = copy.deepcopy(fx.rows[0])
        probe_bad.update(probe={"id": "negative-rtol", "must_refuse": True, "components": ["drv.b"]})
        fx.probes = [probe_ok, probe_bad]
        _, classification, _ = fx.check(self.tmp)
        self.assertEqual(component(classification, "drv.a")["criteria"]["c3"]["status"], "pass")
        b = component(classification, "drv.b")
        self.assertEqual(b["criteria"]["c3"]["status"], "fail")
        self.assertEqual(b["label"], "PRUNE")
        self.assertIsNone(component(classification, "la.x")["criteria"]["c3"]["status"])

    def test_missing_c2_and_allocation_are_insufficient_not_pass(self):
        fx = pair_fixture()
        fx.alloc = []
        _, classification, _ = fx.check(self.tmp)
        a = component(classification, "drv.a")
        self.assertIsNone(a["criteria"]["allocation"]["status"])
        self.assertNotEqual(a["label"], "PROMOTE")
        self.assertTrue(any(s.startswith("allocation: insufficient_evidence") for s in a["insufficient_evidence"]))

    def test_unique_capability_external_gap_optimize(self):
        fx = Fixture()
        fx.component("drv.u", "drv.u", unique_capability=True,
                     external_peers=["ext.scipy-radau", "ext.hairer-rodas"])
        fx.problem("P1")
        # C2 fails (R_up = 20), so drv.u is not PROMOTE; its wins have no holdout, so no SPECIALIZE.
        fx.arm("U", ["drv.u"], "P1", ir=1000, err_scale=20.0)
        for row in fx.rows:
            row["counters"].update(rhs_evaluations=300, jacobian_builds=30, direct_factorizations=30)
        for rtol in RTOLS:  # external rows: one nominal-h0 row per cell is admissible by itself
            ext = fx.row("ext.scipy-radau", [], "P1", rtol, 1.0, rtol)
            ext.update(counters=None, extra_work={"nfev": 100, "njev": 10, "nlu": 10}, counter_semantics="external")
            nat = fx.row("ext.hairer-rodas", [], "P1", rtol, 1.0, rtol)
            nat.update(counters=None, extra_work={"nfev": 50, "njev": 5, "nlu": None}, counter_semantics="external")
        results, classification, _ = fx.check(self.tmp)
        u = component(classification, "drv.u")
        gap = u["criteria"]["optimize"]["external_gap"]
        self.assertEqual((gap["median_gap"], gap["matched_cells"]), (3.0, 1))  # 360 / 120 at target 1e-3
        self.assertEqual(gap["units"], ["rhs_evaluations", "jacobian_evaluations", "lu_factorizations"])
        self.assertEqual(u["label"], "OPTIMIZE", u["rule_path"])
        self.assertEqual(rules(u)[-1], "OPTIMIZE_EXTERNAL_GAP")
        cells = {(g["peer"], g["target"]): g for g in results["external_gaps"]}
        self.assertEqual(cells[("ext.scipy-radau", 1e-3)]["gap"], 3.0)
        self.assertIn("not matched: arm inadmissible", cells[("ext.scipy-radau", 1e-5)]["reason"])
        rodas = cells[("ext.hairer-rodas", 1e-3)]
        self.assertIsNone(rodas["gap"])
        self.assertIn("unit unavailable: lu_factorizations (extra_work.nlu)", rodas["reason"])
        group_ext = group(results, "ext.scipy-radau", "P1", 1e-4)
        self.assertEqual([t["state"] for t in group_ext["targets"]], ["admissible", "inadmissible"])

    def test_c2_failure_blocks_promote(self):
        fx = Fixture()
        fx.component("drv.a", "drv")
        fx.component("drv.b", "drv")
        fx.problem("P1")
        fx.arm("A", ["drv.a"], "P1", ir=1000, err_scale=20.0)  # R_up = 20 > ratio 10 on every group
        fx.arm("B", ["drv.b"], "P1", ir=2000)
        _, classification, _ = fx.check(self.tmp)
        a = component(classification, "drv.a")
        self.assertEqual(a["criteria"]["c2"]["status"], "fail")
        self.assertEqual(a["criteria"]["c2"]["groups"], 2)
        self.assertEqual(a["criteria"]["c2"]["within_ratio"], 0)
        self.assertFalse(a["criteria"]["promote"]["checks"]["c2"])
        self.assertNotEqual(a["label"], "PROMOTE")
        self.assertIn("NOT_PROMOTE", rules(a))

    def test_optimize_via_attribution_headroom(self):
        fx = pair_fixture(cost_a=1000, cost_b=2000)
        fx.attribution = {"schema": "vigilode-triage-attribution-v1", "entries": [
            {"component": "drv.b", "arm": "B", "problem": pid, "grid": "endpoint", "rtol": rtol,
             "removable_ir": 1000, "basis": "allocation and validation overhead (test)"}
            for pid in ("P1", "P2") for rtol in RTOLS]}
        _, classification, _ = fx.check(self.tmp)
        b = component(classification, "drv.b")
        self.assertEqual(b["label"], "OPTIMIZE", b["rule_path"])
        self.assertEqual(rules(b)[-1], "OPTIMIZE_HEADROOM")
        headroom = b["criteria"]["optimize"]["headroom"]
        self.assertEqual((headroom["status"], headroom["adjusted_max"]), ("pass", 1.0))
        fx.attribution["entries"] = fx.attribution["entries"][:1]  # one LOSS cell without attribution
        _, classification, _ = fx.check(self.tmp / "partial")
        b = component(classification, "drv.b")
        self.assertEqual((b["label"], rules(b)[-1]), ("PRUNE", "PRUNE_DEFAULT"))
        self.assertIn("insufficient_evidence", b["criteria"]["optimize"]["headroom"]["reason"])

    def test_invalid_inputs_fail_closed(self):
        fx = pair_fixture()
        fx.rows.append(copy.deepcopy(fx.rows[0]))  # duplicate cell
        argv = fx.write(self.tmp)
        self.assertEqual(tch.main(argv), 2)
        self.assertFalse((self.tmp / "RESULTS.json").exists())

    def test_outputs_are_byte_deterministic_and_immutable(self):
        fx = self.specialize_fixture(False)
        fx.check(self.tmp / "one")
        fx.check(self.tmp / "two")
        for name in ("RESULTS.json", "CLASSIFICATION.json"):
            self.assertEqual((self.tmp / "one" / name).read_bytes(), (self.tmp / "two" / name).read_bytes())
        argv = fx.write(self.tmp / "one")
        with self.assertRaises(SystemExit):
            tch.main(argv)  # refuses to overwrite without --force


class ProfileDeterminismTests(unittest.TestCase):
    def test_exact_flag_or_relative_gap_within_tolerance(self):
        exact = {"deterministic": True, "ir": {"1": 10, "1b": 10}}
        near = {"deterministic": False, "ir": {"1": 846643399, "1b": 846643395}}
        far = {"deterministic": False, "ir": {"1": 1000, "1b": 900}}
        self.assertTrue(tch.profile_is_deterministic(exact, 0.0))
        self.assertFalse(tch.profile_is_deterministic(near, 0.0))
        self.assertTrue(tch.profile_is_deterministic(near, 1e-6))
        self.assertFalse(tch.profile_is_deterministic(far, 1e-6))
        self.assertFalse(tch.profile_is_deterministic(None, 1e-6))
        self.assertFalse(tch.profile_is_deterministic({"deterministic": None, "ir": {"1": None}}, 1e-6))


if __name__ == "__main__":
    unittest.main()


class StateSidecarTests(unittest.TestCase):
    """States of large rows in sidecars: the checker's three accuracy sources agree bit for bit."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="triage_sidecar_")
        self.tmp = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def fixture(self) -> Fixture:
        fx = Fixture()
        fx.component("drv.a", "drv")
        fx.component("drv.b", "drv")
        fx.problem("BIG", dimension=100)  # > the default inline maximum 64
        fx.problem("SMALL", dimension=3)
        for pid in ("BIG", "SMALL"):
            fx.arm("A", ["drv.a"], pid, ir=1000, err_scale=0.37)
            fx.arm("B", ["drv.b"], pid, ir=1500, err_scale=0.21)
        return fx

    def externalize(self, fx: Fixture, states_dir: Path, max_dimension: int = 64) -> None:
        import triage_common as tc
        refs = {(r["problem"], r["grid"]): r for r in fx.refs}
        ext = tc.StateExternalizer(states_dir, max_dimension, refs, fx.problems)
        fx.rows = [ext(r) for r in fx.rows]

    def test_sidecar_round_trip_and_refusal(self):
        import gzip
        import triage_common as tc
        states = [[1.0, -2.5, 3.25e-300], [float(i) for i in range(3)]]
        digest, name = tc.write_states_sidecar(self.tmp, states)
        data = (self.tmp / name).read_bytes()
        self.assertEqual(name, f"{digest}.f64le.gz")
        self.assertEqual(data[4:8], b"\0\0\0\0")  # gzip -n: no mtime
        self.assertEqual(tc.sha256_bytes(gzip.decompress(data)), digest)
        self.assertEqual(gzip.decompress(data)[:8], struct.pack("<d", 1.0))  # little-endian, row-major
        row = {"states_sha256": digest, "states_shape": [2, 3], "states_file": name}
        self.assertEqual(tc.load_states_sidecar([self.tmp], row), (states, "sidecar", None))
        self.assertEqual(tc.write_states_sidecar(self.tmp, states), (digest, name))  # content addressed
        tampered = list(states[0]) + [9.0, 1.0, 2.0]
        (self.tmp / name).write_bytes(gzip.compress(struct.pack("<6d", *tampered), mtime=0))
        loaded, source, reason = tc.load_states_sidecar([self.tmp], row)
        self.assertIsNone(loaded)
        self.assertEqual(source, "sidecar_refused")
        self.assertIn("sha256", reason)
        self.assertEqual(tc.load_states_sidecar([self.tmp], dict(row, states_shape=[3, 3]))[1], "sidecar_refused")
        self.assertEqual(tc.load_states_sidecar([self.tmp / "nowhere"], row)[1], "sidecar_missing")

    def test_dimension_threshold(self):
        import triage_common as tc
        fx = self.fixture()
        refs = {(r["problem"], r["grid"]): r for r in fx.refs}
        big = next(r for r in fx.rows if r["problem"] == "BIG")
        small = next(r for r in fx.rows if r["problem"] == "SMALL")
        self.assertIn("states_hex", tc.StateExternalizer(self.tmp, 100, refs, fx.problems)(big))  # 100 <= 100
        moved = tc.StateExternalizer(self.tmp, 99, refs, fx.problems)(big)
        self.assertNotIn("states_hex", moved)
        self.assertEqual(moved["states_shape"], [2, 100])
        self.assertTrue((self.tmp / moved["states_file"]).is_file())
        self.assertEqual(moved["accuracy_inline"]["reference_sha256"], tc.reference_entry_sha256(refs[("BIG", "endpoint")]))
        self.assertIs(tc.StateExternalizer(self.tmp, 64, refs, fx.problems)(small), small)
        self.assertIs(tc.StateExternalizer(None, 64, refs, fx.problems)(big), big)  # no states dir: kept inline

    def test_inline_accuracy_equals_checker_recomputation(self):
        hex_results, hex_class, _ = self.fixture().check(self.tmp / "hex")
        fx = self.fixture()
        states_dir = self.tmp / "states"
        self.externalize(fx, states_dir)
        argv = fx.write(self.tmp / "sidecar") + ["--states-dir", str(states_dir)]
        self.assertEqual(tch.main(argv), 0)
        side_results = json.loads((self.tmp / "sidecar" / "RESULTS.json").read_text())
        inline_dir = self.tmp / "inline"
        fx.check(inline_dir)  # no --states-dir: accuracy_inline
        inline_results = json.loads((inline_dir / "RESULTS.json").read_text())
        inline_class = json.loads((inline_dir / "CLASSIFICATION.json").read_text())
        norms = ("class", "E", "E_rho", "E_up", "R", "R_rho", "R_up", "reason", "integrity")
        for a, b, c in zip(hex_results["rows"], side_results["rows"], inline_results["rows"]):
            self.assertEqual({k: a[k] for k in norms}, {k: b[k] for k in norms})
            self.assertEqual({k: a[k] for k in norms}, {k: c[k] for k in norms})
            big = a["problem"] == "BIG"
            self.assertEqual(a["accuracy_source"], "recomputed_hex")
            self.assertEqual(b["accuracy_source"], "recomputed_sidecar" if big else "recomputed_hex")
            self.assertEqual(c["accuracy_source"], "runner_inline" if big else "recomputed_hex")
        self.assertEqual(inline_results["accuracy_summary"]["runner_inline"], 12)
        self.assertEqual(inline_results["accuracy_summary"]["sidecars_missing"], 12)
        self.assertEqual(side_results["accuracy_summary"]["recomputed_sidecar"], 12)
        self.assertEqual([e["label"] for e in hex_class["components"]], [e["label"] for e in inline_class["components"]])
        self.assertEqual(hex_results["comparisons"], inline_results["comparisons"])
        import evidence_schema_v2 as es
        es.validate_triage(inline_dir / "RESULTS.json", inline_dir / "CLASSIFICATION.json")

    def test_missing_or_refused_sidecar_uses_flagged_inline(self):
        import gzip
        fx = self.fixture()
        states_dir = self.tmp / "states"
        self.externalize(fx, states_dir)
        files = sorted({r["states_file"] for r in fx.rows if "states_file" in r})
        (states_dir / files[0]).unlink()                                            # one sidecar missing
        (states_dir / files[1]).write_bytes(gzip.compress(b"\0" * 1600, mtime=0))   # one tampered
        argv = fx.write(self.tmp / "out") + ["--states-dir", str(states_dir)]
        self.assertEqual(tch.main(argv), 0)
        results = json.loads((self.tmp / "out" / "RESULTS.json").read_text())
        summary = results["accuracy_summary"]
        missing = sum(1 for r in fx.rows if r.get("states_file") == files[0])
        refused = sum(1 for r in fx.rows if r.get("states_file") == files[1])
        self.assertEqual((summary["sidecars_missing"], summary["sidecars_refused"]), (missing, refused))
        self.assertEqual(summary["runner_inline"], missing + refused)
        self.assertEqual(summary["recomputed_sidecar"], 12 - missing - refused)

    def test_no_states_and_no_inline_is_null_with_reason(self):
        fx = self.fixture()
        self.externalize(fx, self.tmp / "states")
        for row in fx.rows:
            row.pop("accuracy_inline", None)
        fx.check(self.tmp / "out")
        results = json.loads((self.tmp / "out" / "RESULTS.json").read_text())
        big = [r for r in results["rows"] if r["problem"] == "BIG"]
        self.assertEqual({r["accuracy_source"] for r in big}, {"none"})
        self.assertEqual({r["class"] for r in big}, {"unknown"})
        self.assertIsNone(big[0]["E"])
        self.assertIn("accuracy: null: no states and no accuracy_inline", big[0]["reason"])

    def test_inline_bound_to_its_reference(self):
        fx = self.fixture()
        self.externalize(fx, self.tmp / "states")
        big_ref = next(r for r in fx.refs if r["problem"] == "BIG")
        big_ref["rho_hex"][1] = [hx(1e-9)] * 100  # the checker's reference differs from the runner's
        fx.check(self.tmp / "out")
        results = json.loads((self.tmp / "out" / "RESULTS.json").read_text())
        big = [r for r in results["rows"] if r["problem"] == "BIG"]
        self.assertEqual({r["accuracy_source"] for r in big}, {"none"})
        self.assertIn("computed against a different reference", big[0]["reason"])

    def test_gz_references_in_the_checker(self):
        import gzip
        fx = self.fixture()
        argv = fx.write(self.tmp / "gz")
        refs = self.tmp / "gz" / "REFERENCES.json"
        gz = self.tmp / "gz" / "REFERENCES.json.gz"
        gz.write_bytes(gzip.compress(refs.read_bytes(), mtime=0))
        argv[argv.index(str(refs))] = str(gz)
        self.assertEqual(tch.main(argv), 0)
        results = json.loads((self.tmp / "gz" / "RESULTS.json").read_text())
        self.assertEqual(results["inputs"]["references"]["sha256"],
                         __import__("hashlib").sha256(refs.read_bytes()).hexdigest())  # digest of the content
        self.assertEqual({r["accuracy_source"] for r in results["rows"]}, {"recomputed_hex"})
