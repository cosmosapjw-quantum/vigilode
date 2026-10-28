#!/usr/bin/env python3
"""Tests of the component-triage tools other than the runner, profiler and decision engine:
tools/triage/{triage_manifest,triage_external,triage_references,triage_parity,triage_report}.py and host_prep.sh.

Fakes come from tools/test_triage_runner.py. Tests that need NumPy/SciPy (references, parity, SciPy arms) skip
cleanly when those are not importable.

Run: python3 -m pytest tools/test_triage_tools.py -q
"""

from __future__ import annotations

import copy
import importlib.util
import json
import struct
import subprocess
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
from test_triage_runner import CORPUS, LIST, FakeEnv, read_rows, write_executable  # noqa: E402

HAVE_SCIPY = importlib.util.find_spec("numpy") is not None and importlib.util.find_spec("scipy") is not None
needs_scipy = unittest.skipUnless(HAVE_SCIPY, "numpy/scipy not installed")


def components_for(list_doc: dict, **overrides) -> dict:
    comps = []
    for arm in list_doc["arms"]:
        for cid in arm["components"]:
            if any(c["id"] == cid for c in comps):
                continue
            comps.append({"id": cid, "family": "F1", "slot": cid.split(".")[0], "files": [], "reach": "D",
                          "selector": "", "capability": "", "tiers": ["G", "P"], "arms": [arm["id"]], "prior": [],
                          "blocked_by": [], "loc": 10, "static_class": None, "runtime_dependents": [],
                          "unique_capability": False, "notes": ""})
    for c in comps:
        c.update(overrides.get(c["id"], {}))
    return {"schema": "vigilode-triage-components-v1", "base_commit": "0" * 40, "components": comps}


class Tmp(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="triage_tools_")
        self.fake = FakeEnv(Path(self._tmp.name))
        self.root = self.fake.root

    def tearDown(self):
        self._tmp.cleanup()


# ------------------------------------------------------------------------------------------ manifest
class ManifestTests(Tmp):
    def manifest(self, comps: dict, *extra: str):
        path = self.root / "COMPONENTS.json"
        path.write_text(json.dumps(comps))
        return self.fake.run("triage_manifest.py", "--components", str(path), "--corpus", str(self.fake.corpus),
                             "--list", str(self.fake.list), *extra)

    def test_valid_manifest_and_dry_run_counts(self):
        comps = components_for(LIST, **{"drv.banded": {"reach": "S"}, "la.banded": {"reach": "S"},
                                        "drv.unwired": {"reach": "T"}})
        proc = self.manifest(comps, "--dry-run", "--seconds-per-cell", "runs=1,profile=36,alloc=3.6")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        report = json.loads(proc.stdout)
        self.assertEqual(report["errors"], [])
        self.assertFalse(report["applicability"]["banded-arm"]["robertson"]["applicable"])
        self.assertIn("structure", report["applicability"]["banded-arm"]["robertson"]["reason"])
        # 4 applicable wired arms x 2 problems x 2 rtols x 1 grid x 3 h0; subset: robertson, rtol 1e-6, h0 1
        self.assertIn("runs: 48 cells, ~0.01 h at 1 s/cell", proc.stderr)
        self.assertIn("profile: 4 cells, ~0.04 h at 36 s/cell", proc.stderr)
        self.assertIn("alloc: 4 cells", proc.stderr)
        self.assertIn("unwired-arm", proc.stderr)

    def test_errors_exit_nonzero(self):
        comps = components_for(LIST, **{"drv.unwired": {"reach": "T"}, "la.banded": {"reach": "S"}})
        comps["components"][0]["arms"].append("no-such-arm")
        comps["components"].append(copy.deepcopy(comps["components"][1]))
        proc = self.manifest(comps)
        self.assertEqual(proc.returncode, 1)
        errors = json.loads(proc.stdout)["errors"]
        text = "\n".join(errors)
        self.assertIn("unknown arm id 'no-such-arm'", text)
        self.assertIn("duplicate component id", text)
        # drv.banded (reach D): its only wired arm has no applicable CORPUS problem (no banded problem)
        self.assertIn("coverage gap: component drv.banded", text)

    def test_static_reach_on_a_small_repository(self):
        repo = self.root / "repo"
        files = {
            "crates/rodas5p-integrators/src/lib.rs": "mod drv;\nmod helper;\nmod dead;\npub use drv::drive_it;\n",
            "crates/rodas5p-integrators/src/drv.rs": "use rodas5p_core::core_thing_abc;\n"
                                                     "pub fn drive_it() { super::helper::help_me(); core_thing_abc(); }\n",
            "crates/rodas5p-integrators/src/helper.rs": "pub fn help_me() {}\n#[cfg(test)]\nmod tests {\n"
                                                        "    fn t() { crate::dead::never_called_xyz(); }\n}\n",
            "crates/rodas5p-integrators/src/dead.rs": "pub fn never_called_xyz() {}\n",
            "crates/rodas5p-integrators/tests/it.rs": "fn x() { rodas5p_integrators::dead::never_called_xyz(); }\n",
            "crates/rodas5p-core/src/lib.rs": "pub fn core_thing_abc() {}\n",
            "crates/rodas5p-krylov/src/lib.rs": "// rodas5p_integrators::dead::never_called_xyz in a comment\n",
        }
        for rel, text in files.items():
            (repo / rel).parent.mkdir(parents=True, exist_ok=True)
            (repo / rel).write_text(text)
        subprocess.run(["git", "init", "-q", str(repo)], check=True)
        listing = {"schema": "vigilode-triage-list-v1", "problems": LIST["problems"],
                   "arms": [{"id": "drv-arm", "components": ["drv.x"], "wired": True, "unwired_reason": None,
                             "requires": {"jacobian": "any", "structure": None, "mass": False, "fixed_dimension": None}}]}
        list_path = self.root / "reach_list.json"
        list_path.write_text(json.dumps(listing))
        comps = components_for(listing, **{"drv.x": {"files": ["crates/rodas5p-integrators/src/drv.rs"]}})
        comps_path = self.root / "COMPONENTS.json"
        comps_path.write_text(json.dumps(comps))
        out = self.root / "STATIC_REACH.json"
        proc = self.fake.run("triage_manifest.py", "--components", str(comps_path), "--corpus", str(self.fake.corpus),
                             "--list", str(list_path), "--static-reach", str(out), "--repo", str(repo))
        self.assertEqual(proc.returncode, 0, proc.stderr)
        reach = {m["file"].split("/src/")[-1] if "integrators" in m["file"] else m["file"]: m
                 for m in json.loads(out.read_text())["modules"]}
        self.assertTrue(reach["drv.rs"]["reachable_from_wired_arm"])
        self.assertEqual(reach["helper.rs"]["via"], "crates/rodas5p-integrators/src/drv.rs")
        self.assertTrue(reach["crates/rodas5p-core/src/lib.rs"]["reachable_from_wired_arm"])
        self.assertFalse(reach["dead.rs"]["reachable_from_wired_arm"])
        self.assertEqual(reach["dead.rs"]["non_test_callers"], [])  # test file, cfg(test) tail and comment ignored
        self.assertEqual(reach["drv.rs"]["non_test_callers"], ["crates/rodas5p-integrators/src/lib.rs"])
        proc = self.fake.run("triage_manifest.py", "--components", str(comps_path), "--corpus", str(self.fake.corpus),
                             "--list", str(list_path), "--static-reach", str(out), "--repo", str(repo))
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("immutable output exists", proc.stderr)


# ------------------------------------------------------------------------------------------ external
FAKE_NATIVE = r'''
import json, sys
_, cmd, problem, arm, rtol, reps, warm = sys.argv
if problem not in ("robertson",):
    sys.stderr.write(f"unknown problem {problem}\n"); sys.exit(2)
rtol = float(rtol)
print("note from a Fortran code")
print(json.dumps({"problem": problem, "arm": arm, "rtol": rtol, "atol": rtol * 1.0e-4, "status": "completed",
                  "message": "", "final_state": [0.7158, 9.185e-06, 0.2842], "accepted_steps": 80,
                  "rejected_steps": 3, "counters": {"rhs_evaluations": 700, "jacobian_builds": 40,
                  "direct_factorizations": 90}, "wall_seconds": [0.001], "wall_median": 0.001,
                  "deterministic": True}))
'''


class ExternalTests(Tmp):
    def external(self, *extra: str, corpus: Path | None = None, out: Path | None = None):
        return self.fake.run("triage_external.py", "--corpus", str(corpus or self.fake.corpus), "--campaign",
                             "ct-fake", "--output", str(out or self.root / "EXTERNAL.jsonl"), "--problem", "robertson",
                             *extra)

    def test_unavailable_rows_are_typed_never_skipped(self):
        proc = self.external("--external-arm", "ext.cvode", "--external-arm", "ext.diffsol-bdf")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        rows = read_rows(self.root / "EXTERNAL.jsonl")
        self.assertEqual(len(rows), 2 * 2)  # 2 arms x 2 rtols, endpoint grid, the single nominal h0 = 1 row
        self.assertEqual({r["h0_scale"] for r in rows}, {1.0})
        self.assertEqual({r["status"] for r in rows}, {"unavailable"})
        self.assertTrue(all(r["error"] for r in rows))
        self.assertIn("native driver not given", rows[0]["error"])
        self.assertTrue(any("Diffsol harness binary not given" in r["error"] for r in rows))
        import triage_check as tch
        for r in rows:
            self.assertEqual(tch.validate_row(r, "external"), [])
            self.assertIsNone(r["counters"])

    @unittest.skipUnless(HAVE_SCIPY, "native endpoint rows take the initial state from the NumPy model")
    def test_native_driver_rows(self):
        driver = write_executable(self.root / "native_stiff", FAKE_NATIVE)
        proc = self.external("--external-arm", "ext.hairer-rodas", "--native-driver", str(driver))
        self.assertEqual(proc.returncode, 0, proc.stderr)
        rows = {(r["rtol"], r["h0_scale"]): r for r in read_rows(self.root / "EXTERNAL.jsonl")}
        self.assertEqual(sorted(rows), [(1e-6, 1.0), (1e-4, 1.0)])
        ok = rows[(1e-4, 1.0)]
        self.assertEqual(ok["status"], "ok", ok["error"])
        self.assertEqual(ok["output_times"], [0.0, 40.0])
        self.assertEqual(ok["states_hex"][1][0], struct.pack(">d", 0.7158).hex())
        self.assertEqual((ok["extra_work"]["nfev"], ok["extra_work"]["nlu"]), (700, 90))
        self.assertIsNone(ok["h0"])
        self.assertNotIn("wall_seconds", ok["extra_work"]["raw"])
        self.assertEqual(ok["extra_work"]["h0_policy"], "solver_default")

    @needs_scipy
    def test_external_rows_use_state_sidecars(self):
        import triage_common as tc
        out, states = self.root / "ext_sidecar.jsonl", self.root / "states"
        proc = self.external("--external-arm", "ext.scipy-radau", "--rtol", "1e-04", "--states-dir", str(states),
                             "--inline-states-max-dimension", "2", out=out)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        row = read_rows(out)[0]
        self.assertEqual(row["status"], "ok", row["error"])
        self.assertNotIn("states_hex", row)
        self.assertEqual(row["states_shape"], [2, 3])
        self.assertEqual(tc.load_states_sidecar([states], row)[0][0], [1.0, 0.0, 0.0])
        self.assertEqual(row["accuracy_inline"]["reason"], "reference: reference_missing")
        import triage_check as tch
        self.assertEqual(tch.validate_row(row, "external"), [])

    @needs_scipy
    def test_scipy_radau_row(self):
        proc = self.external("--external-arm", "ext.scipy-radau", "--rtol", "1e-04", "--h0-scale", "1.0",
                             "--repetitions", "2")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        row = read_rows(self.root / "EXTERNAL.jsonl")[0]
        self.assertEqual(row["status"], "ok", row["error"])
        self.assertTrue(row["deterministic"])
        self.assertGreater(row["extra_work"]["nfev"], 0)
        self.assertEqual(row["output_times"], [0.0, 40.0])
        y40 = struct.unpack(">d", bytes.fromhex(row["states_hex"][1][0]))[0]
        self.assertAlmostEqual(y40, 0.7158, delta=1e-3)
        self.assertNotIn("wall", json.dumps(row))


# ----------------------------------------------------------------------------- references and parity
class ReferenceTests(Tmp):
    def corpus_with(self, problems: list[dict]) -> Path:
        corpus = copy.deepcopy(CORPUS)
        corpus["problems"] = problems
        path = self.root / "corpus_ref.json"
        path.write_text(json.dumps(corpus))
        return path

    @needs_scipy
    def test_two_method_reference_and_null_entries(self):
        robertson = copy.deepcopy(CORPUS["problems"][0])
        ghost = dict(copy.deepcopy(robertson), id="ghost")
        exact = dict(copy.deepcopy(robertson), id="exactly", reference={"kind": "exact"})
        out = self.root / "REFERENCES.json"
        proc = self.fake.run("triage_references.py", "--corpus", str(self.corpus_with([robertson, ghost, exact])),
                             "--output", str(out))
        self.assertEqual(proc.returncode, 0, proc.stderr)
        doc = json.loads(out.read_text())
        refs = {r["problem"]: r for r in doc["references"]}
        r = refs["robertson"]
        self.assertEqual((r["status"], r["kind"], r["canonical_method"]), ("ok", "two_method", "Radau"))
        self.assertEqual(r["output_times"], [0.0, 40.0])
        y = struct.unpack(">d", bytes.fromhex(r["states_hex"][1][0]))[0]
        rho = struct.unpack(">d", bytes.fromhex(r["rho_hex"][1][0]))[0]
        self.assertAlmostEqual(y, 0.7158270687, delta=1e-8)
        self.assertGreaterEqual(rho, 0.0)
        self.assertLess(rho, 1e-8)
        self.assertEqual([m["method"] for m in r["methods"]], ["Radau", "LSODA"])
        self.assertEqual(r["methods"][0]["rtol"], 1e-13)
        self.assertEqual(r["methods"][0]["atol"], 1e-17)
        self.assertEqual((refs["ghost"]["status"], refs["ghost"]["reason"]), ("null", "no_numpy_model"))
        self.assertEqual(refs["exactly"]["reason"], "exact_requires_binary")
        self.assertEqual(set(doc["runtime"]), {"python", "numpy", "scipy", "machine"})
        proc = self.fake.run("triage_references.py", "--corpus", str(self.root / "corpus_ref.json"), "--output",
                             str(out))
        self.assertNotEqual(proc.returncode, 0)  # immutable output

    @needs_scipy
    def test_exact_reference_from_the_binary(self):
        exact = dict(copy.deepcopy(CORPUS["problems"][0]), id="exactly", reference={"kind": "exact",
                                                                                     "methods": ["exact"]})
        corpus = self.corpus_with([exact])
        for shift, status in (("", "ok"), ("1", "null")):
            out = self.root / f"REFERENCES_exact{shift}.json"
            proc = self.fake.run("triage_references.py", "--corpus", str(corpus), "--binary", str(self.fake.binary),
                                 "--output", str(out), extra_env={"FAKE_EXACT_SHIFT": shift} if shift else None)
            self.assertEqual(proc.returncode, 0, proc.stderr)
            refs = {(r["problem"], r["grid"]): r for r in json.loads(out.read_text())["references"]}
            ref = refs[("exactly", "endpoint")]  # the test CORPUS sweeps the endpoint grid; the shift moves tf
            self.assertEqual(ref["status"], status, ref["reason"])
        self.assertIn("differ from the reference grid (bitwise)", ref["reason"])
        out = self.root / "REFERENCES_exact.json"
        ref = {(r["problem"], r["grid"]): r for r in json.loads(out.read_text())["references"]}[("exactly", "endpoint")]
        self.assertEqual(ref["rho_hex"], [[struct.pack(">d", 0.0).hex()] * 3] * 2)
        self.assertEqual(ref["states_hex"][1][0], struct.pack(">d", __import__("math").exp(-40.0)).hex())

    @needs_scipy
    def test_corpus_method_names(self):
        robertson = dict(copy.deepcopy(CORPUS["problems"][0]),
                         reference={"kind": "two_method", "methods": ["scipy-radau", "scipy-lsoda"]})
        bogus = dict(copy.deepcopy(robertson), id="hires", dimension=8,
                     reference={"kind": "two_method", "methods": ["scipy-radau", "matlab-ode15s"]})
        out = self.root / "REFERENCES_names.json"
        proc = self.fake.run("triage_references.py", "--corpus", str(self.corpus_with([robertson, bogus])),
                             "--output", str(out), "--grid", "endpoint")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        refs = {r["problem"]: r for r in json.loads(out.read_text())["references"]}
        self.assertEqual([m["method"] for m in refs["robertson"]["methods"]], ["Radau", "LSODA"])
        self.assertIn("unknown reference method(s) ['matlab-ode15s']", refs["hires"]["reason"])

    @needs_scipy
    def test_parity_real_layouts(self):
        thresholds = self.root / "NO_THRESHOLDS.json"
        cases = {  # env -> (exit, robertson status, compared kinds, reason needle)
            "dense": ({}, 0, "pass", ["rhs", "jacobian"], None),
            "coo": ({"FAKE_PARITY_MODE": "coo"}, 0, "pass", ["rhs", "jacobian"], None),
            "jvp": ({"FAKE_PARITY_MODE": "jvp"}, 0, "pass", ["rhs", "jvp"], None),
            "f_error": ({"FAKE_PARITY_MODE": "f_error"}, 1, "fail", ["rhs", "jacobian"],
                        "Rust f failed (callback error: state outside the domain) but the model f succeeded"),
            "y0": ({"FAKE_PARITY_Y0_SHIFT": "1"}, 1, "fail", None, "sample 0 state differs"),
            "mass": ({"FAKE_PARITY_MASS": "1"}, 1, "fail", None, "mass matrix on one side only"),
            "perturbed": ({"FAKE_PARITY_PERTURB": "1e-10"}, 1, "fail", None, "rhs max rel"),
        }
        for name, (env, code, status, compared, needle) in cases.items():
            with self.subTest(case=name):
                out = self.root / f"PARITY_{name}.json"
                proc = self.fake.run("triage_parity.py", "--binary", str(self.fake.binary), "--corpus",
                                     str(self.fake.corpus), "--thresholds", str(thresholds), "--output", str(out),
                                     "--problem", "robertson", extra_env=env)
                self.assertEqual(proc.returncode, code, proc.stderr)
                r = json.loads(out.read_text())["problems"][0]
                self.assertEqual(r["status"], status, r["reason"])
                if compared is not None:
                    self.assertEqual(r["compared"], compared)
                if needle is not None:
                    self.assertIn(needle, r["reason"])
                if name == "perturbed":
                    self.assertEqual((r["worst"]["kind"], r["worst"]["sample"], r["worst"]["index"]), ("rhs", 0, 0))
                if name == "f_error":
                    self.assertEqual(r["f_error_samples"], 2)

    @needs_scipy
    def test_new_numpy_models(self):
        import numpy as np
        import triage_models as tm
        gs = tm.model_for({"id": "gray-scott-16"})
        self.assertEqual(gs.dimension, 512)
        v = np.sin(np.arange(1.0, 513.0))
        scale = abs(gs.jac(0.0, gs.y0)) @ np.abs(v)
        self.assertLess(np.max(np.abs(gs.jac(0.0, gs.y0) @ v - gs.rust_jvp(0.0, gs.y0, v)) / scale), 1e-15)
        self.assertEqual(gs.y0[0] + gs.y0[256], 1.0)  # u + v = 1 at every cell centre
        ma = tm.model_for({"id": "mass-affine"})
        f = ma.rust_rhs(0.3, ma.y0)
        np.testing.assert_allclose(ma.mass @ ma.rhs(0.3, ma.y0), f, rtol=1e-15)
        self.assertIn("y' = M^{-1} f", ma.source)
        bj = tm.model_for({"id": "brusselator-1d-250", "numpy_model": None})
        w = np.sin(np.arange(1.0, 501.0))
        scale = np.abs(bj.jac(0.0, bj.y0)) @ np.abs(w)
        self.assertLess(np.max(np.abs(bj.jac(0.0, bj.y0) @ w - bj.rust_jvp(0.0, bj.y0, w)) / scale), 1e-15)
        alias = tm.model_for({"id": "brusselator-1d-jvp-250", "numpy_model": "brusselator-1d-250"})
        self.assertEqual(alias.dimension, 500)

    @needs_scipy
    def test_parity_limit_from_thresholds(self):
        thresholds = self.root / "THRESHOLDS.json"
        thresholds.write_text(json.dumps({"schema": "vigilode-triage-thresholds-v1",
                                          "parity": {"max_relative_error": 1e-15}}))
        out = self.root / "PARITY_TH.json"
        proc = self.fake.run("triage_parity.py", "--binary", str(self.fake.binary), "--corpus", str(self.fake.corpus),
                             "--thresholds", str(thresholds), "--output", str(out),
                             extra_env={"FAKE_PARITY_PERTURB": "1e-14"})
        doc = json.loads(out.read_text())
        self.assertEqual((doc["limit"], doc["limit_source"]), (1e-15, "THRESHOLDS.json: parity.max_relative_error"))
        self.assertEqual(proc.returncode, 1)  # 1e-14 passes the 1e-13 fallback but not the THRESHOLDS limit
        frozen = json.loads((ROOT / "research/component_triage_20261019/THRESHOLDS.json").read_text())
        self.assertEqual(frozen["parity"]["max_relative_error"], 1e-13)

    @needs_scipy
    def test_parity_pass_fail_and_not_checked(self):
        thresholds = self.root / "NO_THRESHOLDS.json"
        for perturb, expected_exit, expected in (("0", 0, "pass"), ("1e-10", 1, "fail")):
            out = self.root / f"PARITY_{expected}.json"
            proc = self.fake.run("triage_parity.py", "--binary", str(self.fake.binary), "--corpus",
                                 str(self.fake.corpus), "--thresholds", str(thresholds), "--output", str(out),
                                 extra_env={"FAKE_PARITY_PERTURB": perturb})
            self.assertEqual(proc.returncode, expected_exit, proc.stderr)
            doc = json.loads(out.read_text())
            by = {p["problem"]: p for p in doc["problems"]}
            self.assertEqual(by["robertson"]["status"], expected)
            self.assertTrue(by["robertson"]["y0_match"])
            self.assertEqual((by["wide"]["status"], by["wide"]["reason"]), ("not_checked", "no_numpy_model"))
            self.assertEqual(doc["limit"], 1e-13)
            self.assertIn("PARITY_LIMIT", doc["limit_source"])


# ----------------------------------------------------------------------------------- report, host
class PlanTests(unittest.TestCase):
    def test_profile_subset_grids_and_h0(self):
        import triage_common as tc
        corpus = copy.deepcopy(CORPUS)
        corpus["sweep"]["grids"] = ["endpoint", "uniform101"]
        corpus["profile_subset"] = {"rtols": [1e-6], "max_dimension": 100, "grids": ["uniform101"],
                                    "h0_scales": [1.0]}
        cells, _ = tc.plan_cells(LIST, corpus, "profile", arms=["ok-arm"])
        self.assertEqual([(c["problem"], c["grid"], c["h0_scale"]) for c in cells], [("robertson", "uniform101", 1.0)])
        del corpus["profile_subset"]["grids"]
        cells, _ = tc.plan_cells(LIST, corpus, "profile", arms=["ok-arm"])
        self.assertEqual(sorted(c["grid"] for c in cells), ["endpoint", "uniform101"])


class ReportTests(Tmp):
    def test_report_generation_and_check(self):
        import test_triage_check as ttc
        fx = ttc.pair_fixture()
        fx.check(self.root / "check")
        results, classification = self.root / "check" / "RESULTS.json", self.root / "check" / "CLASSIFICATION.json"
        report = self.root / "REPORT_KO.md"
        args = ["--results", str(results), "--classification", str(classification), "--output", str(report)]
        self.assertEqual(self.fake.run("triage_report.py", *args).returncode, 0)
        text = report.read_text()
        sha = __import__("hashlib").sha256(results.read_bytes()).hexdigest()
        self.assertIn(sha, text)
        self.assertIn("### F1", text)
        self.assertIn("`drv.b` | drv | PRUNE", text)
        self.assertEqual(self.fake.run("triage_report.py", *args, "--check").returncode, 0)
        report.write_text(text.replace("PRUNE", "PROMOTE", 1))
        proc = self.fake.run("triage_report.py", *args, "--check")
        self.assertEqual(proc.returncode, 1)
        self.assertIn("differs", proc.stderr)
        doc = json.loads(classification.read_text())
        doc["summary"]["PRUNE"] += 1
        classification.write_text(json.dumps(doc))
        proc = self.fake.run("triage_report.py", *args, "--check")
        self.assertEqual(proc.returncode, 2)
        self.assertIn("INVALID", proc.stderr)


class HostPrepTests(Tmp):
    def test_records_host_and_refuses_overwrite(self):
        out = self.root / "HOST.json"
        script = str(TRIAGE / "host_prep.sh")
        proc = subprocess.run(["bash", script, "--output", str(out)], capture_output=True, text=True, timeout=60)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        doc = json.loads(out.read_text())
        self.assertEqual(doc["schema"], "vigilode-triage-host-v1")
        self.assertIn("wall_clock_admissible", doc)
        self.assertIsInstance(doc["governors"], dict)
        proc = subprocess.run(["bash", script, "--output", str(out)], capture_output=True, text=True, timeout=60)
        self.assertEqual(proc.returncode, 2)
        self.assertEqual(subprocess.run(["bash", script, "--help"], capture_output=True, text=True).returncode, 0)


class HelpTests(unittest.TestCase):
    def test_every_tool_has_help(self):
        for script in sorted(TRIAGE.glob("triage_*.py")):
            if script.name in ("triage_common.py", "triage_models.py"):
                continue
            with self.subTest(script=script.name):
                proc = subprocess.run([sys.executable, str(script), "--help"], capture_output=True, text=True,
                                      timeout=60)
                self.assertEqual(proc.returncode, 0, proc.stderr)
                self.assertIn("usage:", proc.stdout)


class ApplicabilityTests(unittest.TestCase):
    def test_mass_flag_means_accepts_and_listing_is_authoritative(self):
        import triage_common as tc
        plain = {"id": "p", "jacobian": "analytic", "jvp": True, "mass": False, "dimension": 3, "structure": "dense"}
        massy = dict(plain, id="m", mass=True)
        accepts = {"requires": {"jacobian": "analytic", "mass": True}}
        identity_only = {"requires": {"jacobian": "analytic", "mass": False}}
        self.assertTrue(tc.applicable(accepts, plain)[0])
        self.assertTrue(tc.applicable(accepts, massy)[0])
        self.assertTrue(tc.applicable(identity_only, plain)[0])
        self.assertFalse(tc.applicable(identity_only, massy)[0])
        listed = {"requires": {"jacobian": "analytic", "mass": False}, "applicable_problems": ["m"]}
        self.assertTrue(tc.applicable(listed, massy)[0])
        self.assertFalse(tc.applicable(listed, plain)[0])
        self.assertTrue(tc.applicable({"requires": {"fixed_dimension": [2, 3]}}, plain)[0])
        self.assertFalse(tc.applicable({"requires": {"fixed_dimension": [2, 8]}}, plain)[0])


class GzipInputTests(unittest.TestCase):
    def test_gz_rows_parse_and_digest_like_the_plain_file(self):
        import gzip as gz
        import triage_common as tc
        with tempfile.TemporaryDirectory() as d:
            plain = Path(d) / "RUNS.jsonl"
            plain.write_bytes(b'{"a": 1}\n{"b": 2}\n')
            packed = Path(d) / "RUNS.jsonl.gz"
            packed.write_bytes(gz.compress(plain.read_bytes(), mtime=0))
            self.assertEqual(tc.load_jsonl(plain), tc.load_jsonl(packed))
            self.assertEqual(tc.sha256_file(plain), tc.sha256_file(packed))


class TimeoutIntegrityTests(unittest.TestCase):
    def test_timeout_abort_is_not_an_integrity_failure(self):
        import triage_common as tc
        base = {"status": "process_abort", "success": False, "states_hex": [], "output_times": []}
        crash = tc.row_accuracy(dict(base, process={"kind": "abort"}), {"dimension": 1}, None)
        timeout = tc.row_accuracy(dict(base, process={"kind": "timeout"}), {"dimension": 1}, None)
        self.assertEqual(crash["integrity"], "process_abort")
        self.assertIsNone(timeout["integrity"])
        self.assertEqual(timeout["class"], "failed")


if __name__ == "__main__":
    unittest.main()
