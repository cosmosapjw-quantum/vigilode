#!/usr/bin/env python3
"""Controller failure/identity/resource tests; these do not qualify a solver."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from campaign import (METHODS, analyze, capabilities, eligible, make_plan, run_campaign,
                      run_process, source_identity, task_command, validate_config, result_contract_errors, validate_build_receipt, digest, verify_receipt)


HERE = Path(__file__).resolve().parent


class CampaignTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.config = json.loads((HERE / "smoke.json").read_text())
        self.config.update(cases=[15], methods=["dopri5", "rodas5p-mf"], repeats=2,
                           threads=[1, 2, 4], profiling="none")
        self.binary = self.root / "stub.py"
        self.binary.write_text("#!/usr/bin/env python3\nimport json,sys\na=sys.argv\ndef v(k,d):return a[a.index(k)+1] if k in a else d\nprint(json.dumps({'kind':'result','case_id':int(v('--case','15')),'method':v('--method','dopri5'),'parallel_layer':v('--parallel-layer','serial'),'threads':int(v('--threads','1')),'scientific_status':'PASS_SCOPED','speed_comparison_eligible':True,'actual_error':0.,'error_limit':1e-5,'elapsed_seconds':.01}))\n")
        self.binary.chmod(0o755)
        self.source = {"commit": "a" * 40, "tree": "b" * 40, "clean": True,
                       "exploratory_dirty_run": False, "status_porcelain": ""}
        self.host = {"cpu_topology": {"physical_cpu_selection": [0, 1]}, "perf": {"usable": False}}
        self.catalog = {"cases": [{"id": 15, "supported_methods": list(METHODS)}]}

    def test_tampered_receipt_raw_or_identity_rejected(self):
        folder = self.root / "receipt"
        receipt = run_process([sys.executable, str(self.binary)], folder, 5, 1, [])
        task = {"case": 15}
        receipt.update(source=self.source, binary_sha256=digest(self.binary), task=task)
        path = folder / "RECEIPT.json"; path.write_text(json.dumps(receipt))
        verify_receipt(path, self.source, digest(self.binary), task)
        with self.assertRaises(ValueError): verify_receipt(path, self.source, "0" * 64, task)
        with self.assertRaises(ValueError): verify_receipt(path, self.source, digest(self.binary), {"case": 16})
        (folder / "stdout.jsonl").write_text("{}\n")
        with self.assertRaises(ValueError): verify_receipt(path, self.source, digest(self.binary), task)

    def test_stale_build_receipt_is_rejected(self):
        valid = {"exit_code": 0, "source": self.source, "binary_sha256": digest(self.binary)}
        validate_build_receipt(valid, self.source, self.binary)
        stale = copy.deepcopy(valid)
        stale["source"]["commit"] = "c" * 40
        with self.assertRaises(ValueError): validate_build_receipt(stale, self.source, self.binary)

    def test_accuracy_probes_are_cold_solves_separate_from_timing(self):
        self.config.update(accuracy_only=True, accuracy_fractions=[.5, 1.], threads=[1])
        self.catalog["cases"][0]["t_end"] = 8.
        p = make_plan(self.config, self.catalog, self.source, self.binary, self.host)
        self.assertEqual(p["planned_processes"], 4)
        self.assertTrue(all(x["purpose"] == "accuracy-probe" for x in p["tasks"]))
        self.assertIn("--end", task_command(p, p["tasks"][0]))

    def test_dense_dimension_guard_is_applied_before_execution(self):
        self.catalog["cases"][0]["dimension_standard"] = 1000
        self.config.update(size="standard", methods=["rodas5p-dense", "dopri5"])
        p = make_plan(self.config, self.catalog, self.source, self.binary, self.host)
        self.assertTrue(all(x["method"] != "rodas5p-dense" for x in p["tasks"]))

    def test_duplicate_thread_levels_rejected(self):
        self.config["threads"] = [1, 1]
        with self.assertRaises(ValueError): validate_config(self.config)

    def tearDown(self):
        self.temp.cleanup()

    def test_config_rejects_unregistered_threads(self):
        self.config["threads"] = [3]
        with self.assertRaises(ValueError): validate_config(self.config)

    def test_config_rejects_nan_and_invalid_methods(self):
        self.config["rtol"] = float("nan")
        with self.assertRaises(ValueError): validate_config(self.config)
        self.config["rtol"] = 1e-6
        self.config["methods"] = ["cvode"]
        with self.assertRaises(ValueError): validate_config(self.config)

    def test_config_rejects_duplicate_cases(self):
        self.config["cases"] = [1, 1]
        with self.assertRaises(ValueError): validate_config(self.config)

    def test_plan_keeps_serial_and_ensemble_distinct_and_caps_cores(self):
        p = make_plan(self.config, self.catalog, self.source, self.binary, self.host)
        self.assertEqual(p["nominal_case_thread_configurations"], 3)
        self.assertEqual(p["planned_processes"], 12)
        self.assertEqual(len(p["planned_skips"]), 4)
        self.assertEqual({x["parallel_layer"] for x in p["tasks"]}, {"serial", "ensemble"})
        self.assertEqual(len({x["id"] for x in p["tasks"]}), 12)

    def test_method_order_rotates_in_independent_replicates(self):
        p = make_plan(self.config, self.catalog, self.source, self.binary, self.host)
        a = [x["method"] for x in p["tasks"] if x["repeat"] == 0 and x["parallel_layer"] == "serial"]
        b = [x["method"] for x in p["tasks"] if x["repeat"] == 1 and x["parallel_layer"] == "serial"]
        self.assertEqual(a, list(reversed(b)))

    def test_profiles_are_separate_and_timers_opt_in(self):
        self.config["profiling"] = "perf-stat"
        p = make_plan(self.config, self.catalog, self.source, self.binary, self.host)
        profile = [x for x in p["tasks"] if x["instrumented"]]
        clean = [x for x in p["tasks"] if not x["instrumented"]]
        self.assertEqual(len(profile), 2)
        self.assertIn("--instrument", task_command(p, profile[0]))
        self.assertNotIn("--instrument", task_command(p, clean[0]))

    def test_raw_success_has_hashes_and_resource_receipt(self):
        r = run_process([sys.executable, str(self.binary)], self.root / "run", 5, 1, [])
        self.assertEqual(r["execution_status"], "COMPLETED")
        self.assertEqual(len(r["stdout_sha256"]), 64)
        self.assertEqual(r["result"]["actual_error"], 0.)

    def test_process_failure_is_not_solver_pass(self):
        r = run_process([sys.executable, "-c", "raise SystemExit(7)"], self.root / "fail", 5, 1, [])
        self.assertEqual(r["exit_code"], 7)
        self.assertFalse(eligible(r))

    def test_malformed_output_is_retained(self):
        r = run_process([sys.executable, "-c", "print('not json')"], self.root / "bad", 5, 1, [])
        self.assertEqual(r["execution_status"], "MALFORMED_RESULT")
        self.assertTrue(r["parse_errors"])

    def test_timeout_kills_process_group_and_preserves_started_record(self):
        r = run_process([sys.executable, "-c", "import time; time.sleep(5)"], self.root / "timeout", .05, 1, [])
        self.assertEqual(r["execution_status"], "TIMEOUT")
        self.assertEqual(r["exit_code"], 124)
        self.assertTrue((self.root / "timeout/START.json").exists())

    def test_completed_pass_still_needs_actual_reference_error(self):
        r = run_process([sys.executable, str(self.binary)], self.root / "eligible", 5, 1, [])
        self.assertTrue(eligible(r))
        r["result"]["actual_error"] = None
        self.assertFalse(eligible(r))
        r["result"]["actual_error"] = 1e-2
        self.assertFalse(eligible(r))

    def test_profile_and_dirty_evidence_excluded_from_speed_ratios(self):
        r = run_process([sys.executable, str(self.binary)], self.root / "r", 5, 1, [])
        r["task"] = {"instrumented": True}
        self.assertFalse(eligible(r))
        r["task"]["instrumented"] = False
        r["source"] = {"exploratory_dirty_run": True}
        self.assertFalse(eligible(r))

    def test_no_overwrite_existing_attempt(self):
        folder = self.root / "same"
        run_process([sys.executable, str(self.binary)], folder, 5, 1, [])
        with self.assertRaises(FileExistsError):
            run_process([sys.executable, str(self.binary)], folder, 5, 1, [])

    def test_source_identity_uses_exact_tree_and_rejects_dirty(self):
        repo = self.root / "repo"
        subprocess.run(["git", "init", "-q", str(repo)], check=True)
        subprocess.run(["git", "-C", str(repo), "-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit", "--allow-empty", "-qm", "fixture"], check=True)
        identity = source_identity(repo)
        self.assertTrue(identity["clean"])
        with self.assertRaises(ValueError): source_identity(repo, "f" * 40)
        with self.assertRaises(ValueError): source_identity(repo, expected_tree="f" * 40)
        (repo / "untracked").write_text("not included")
        with self.assertRaises(ValueError): source_identity(repo)
        self.assertTrue(source_identity(repo, allow_dirty=True)["exploratory_dirty_run"])

    def test_perf_absence_does_not_claim_profile(self):
        with patch("campaign.shutil.which", return_value=None):
            cap = capabilities()
        self.assertFalse(cap["perf"]["usable"])
        self.assertEqual(cap["perf"]["probe"], "NOT_RUN")

    def test_resume_skips_completed_and_preserves_receipts(self):
        self.config.update(methods=["dopri5"], repeats=1, threads=[1])
        p = make_plan(self.config, self.catalog, self.source, self.binary, self.host)
        for task in p["tasks"]: task["cpus"] = []
        path = self.root / "plan.json"; path.write_text(json.dumps(p))
        with patch("campaign.source_identity", return_value=self.source):
            a = run_campaign(path, self.root / "campaign")
            b = run_campaign(path, self.root / "campaign")
        self.assertEqual(a["executed_now"], 2)
        self.assertEqual(b["executed_now"], 0)
        self.assertEqual(b["resumed_existing"], 2)
        self.assertEqual(len(list((self.root / "campaign").glob("runs/*/attempt_*/RECEIPT.json"))), 2)

    def test_changed_binary_refused_before_execution(self):
        p = make_plan(self.config, self.catalog, self.source, self.binary, self.host)
        path = self.root / "plan.json"; path.write_text(json.dumps(p))
        self.binary.write_text("changed")
        with patch("campaign.source_identity", return_value=self.source):
            with self.assertRaises(ValueError): run_campaign(path, self.root / "out")
        self.assertFalse((self.root / "out").exists())

    def test_wrong_result_identity_rejected(self):
        task = {"case": 15, "method": "dopri5", "parallel_layer": "serial", "threads": 1}
        result = {"case_id": 16, "method": "dopri5", "parallel_layer": "data", "threads": 2}
        self.assertEqual(len(result_contract_errors(result, task)), 3)

    def test_state_emission_is_explicit(self):
        p = make_plan(self.config, self.catalog, self.source, self.binary, self.host)
        self.assertNotIn("--emit-state", task_command(p, p["tasks"][0]))
        p["config"]["emit_state"] = True
        self.assertIn("--emit-state", task_command(p, p["tasks"][0]))

    def test_partial_replicates_not_ranked(self):
        self.config.update(methods=["dopri5"], repeats=2, threads=[1])
        p = make_plan(self.config, self.catalog, self.source, self.binary, self.host)
        p["tasks"] = p["tasks"][:1]
        p["tasks"][0]["cpus"] = []
        path = self.root / "plan.json"; path.write_text(json.dumps(p))
        out = self.root / "partial"
        with patch("campaign.source_identity", return_value=self.source): run_campaign(path, out)
        report = analyze(out)
        data = json.loads(Path(report["analysis"]).read_text())
        self.assertIsNone(data["groups"][0]["seconds_median"])
        self.assertFalse(data["groups"][0]["complete_eligible_group"])


if __name__ == "__main__":
    unittest.main()
