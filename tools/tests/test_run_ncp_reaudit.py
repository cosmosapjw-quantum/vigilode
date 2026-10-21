"""Hostile receipt and protocol contracts; no Cargo, native solver or cloud use."""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import run_ncp_reaudit as runner


class NcpContracts(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.email", "test@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "config", "user.name", "Test"], check=True)
        (self.repo / "source.rs").write_text("fn stable() {}\n")
        subprocess.run(["git", "-C", str(self.repo), "add", "."], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-qm", "fixture"], check=True)
        self.output = self.root / "output"
        self.output.mkdir()
        self.source = runner.source_identity(self.repo)
        self.config = {"memory_limit_bytes": 1024**3, "_execution_identity_sha256": "f" * 64}
        self.case = {"suite": "polynomial", "case": "p4", "original_budget": 1e-7}
        self.row = {"schema": "vigilode.reaudit-comparison.v1", "suite": "polynomial", "case": "p4",
                    "arm": "protected", "threads": 1, "scope": "REQUESTED_ODE_ENDPOINTS_ONLY",
                    "status": "PASS_CERTIFIED_ENDPOINTS", "full_curve_certified": False,
                    "original_budget": 1e-7, "achieved_bound": 1e-8, "elapsed_ns": 10,
                    "input_sha256": "a" * 64}

    def tearDown(self):
        self.temp.cleanup()

    def test_dirty_source_is_detected(self):
        (self.repo / "source.rs").write_text("fn changed() {}\n")
        other = runner.source_identity(self.repo)
        self.assertFalse(other["clean"])
        self.assertNotEqual(other["closure_sha256"], self.source["closure_sha256"])

    def test_untracked_input_is_detected(self):
        (self.repo / "injected.rs").write_text("untracked")
        self.assertFalse(runner.source_identity(self.repo)["clean"])

    def test_tracked_internal_symlink_binds_target_bytes(self):
        (self.repo / "linked.rs").symlink_to("source.rs")
        subprocess.run(["git", "-C", str(self.repo), "add", "linked.rs"], check=True)
        before = runner.source_identity(self.repo)
        (self.repo / "source.rs").write_text("changed target")
        after = runner.source_identity(self.repo)
        self.assertNotEqual(before["closure_sha256"], after["closure_sha256"])

    def test_tracked_external_symlink_rejected(self):
        (self.root / "external.rs").write_text("outside")
        (self.repo / "linked.rs").symlink_to(self.root / "external.rs")
        subprocess.run(["git", "-C", str(self.repo), "add", "linked.rs"], check=True)
        with self.assertRaisesRegex(ValueError, "outside"):
            runner.source_identity(self.repo)

    def test_nested_and_symlink_output_rejected(self):
        with self.assertRaises(ValueError):
            runner.outside_output(self.repo, self.repo / "results")
        (self.root / "alias").symlink_to(self.repo, target_is_directory=True)
        with self.assertRaises(ValueError):
            runner.outside_output(self.repo, self.root / "alias/results")

    def test_missing_cpu_capacity_rejected(self):
        with patch.object(os, "sched_getaffinity", return_value={1, 2}):
            with self.assertRaises(ValueError):
                runner.choose_cpus("auto", 8)
            with self.assertRaises(ValueError):
                runner.choose_cpus("1,7", 2)

    def test_no_install_or_parallel_build_steps(self):
        steps = runner.steps({})
        for _, argv, _ in steps:
            self.assertNotIn("install", argv)
            if argv[0] == "cargo" and argv[1] in ("test", "build", "clippy"):
                self.assertIn("--locked", argv)
                self.assertIn("--offline", argv)
                self.assertIn("-j1", argv)
        self.assertEqual(next(s[2] for s in steps if s[0] == "clippy"), "clippy")
        self.assertNotEqual(next(s[2] for s in steps if s[0] == "clippy"),
                            next(s[2] for s in steps if s[0] == "workspace_tests"))

    def test_native_raw_row_accepts_only_its_scope(self):
        self.assertEqual(runner.numeric_row_issues(self.row, self.case, "protected", 1), [])
        row = dict(self.row, scope="FULL_CURVE", full_curve_certified=True)
        self.assertTrue(runner.numeric_row_issues(row, self.case, "protected", 1))

    def test_refusal_and_budget_inflation_fail(self):
        for updates in ({"status": "REFUSED"}, {"original_budget": 1.0}, {"achieved_bound": 1.0},
                        {"achieved_bound": None}, {"achieved_bound": float("nan")},
                        {"achieved_bound": True}, {"elapsed_ns": 0}, {"input_sha256": None}):
            with self.subTest(updates=updates):
                self.assertTrue(runner.numeric_row_issues(dict(self.row, **updates), self.case, "protected", 1))

    def test_stage_cannot_pass_as_endpoint(self):
        case = {"suite": "actual-k", "case": "k2", "original_budget": 1e-9}
        row = dict(self.row, suite="actual-k", case="k2", original_budget=1e-9, achieved_bound=1e-10)
        self.assertTrue(runner.numeric_row_issues(row, case, "protected", 1))
        row.update(scope="STAGE_FUNCTIONAL_ONLY", status="PASS_CERTIFIED_STAGE_FUNCTIONAL")
        self.assertEqual(runner.numeric_row_issues(row, case, "protected", 1), [])

    def test_unknown_zero_or_excess_rss_fail(self):
        for peak, status in ((None, "UNAVAILABLE"), (0, "OBSERVED"), (2**31, "OBSERVED")):
            receipt = {"process": {"peak_rss_bytes": peak, "rss_status": status, "affinity_verified": True}}
            self.assertTrue(runner.memory_issues(receipt, self.config))

    def test_affinity_escape_and_observed_oom_fail(self):
        base = {"peak_rss_bytes": 1024, "rss_status": "OBSERVED", "affinity_verified": True}
        self.assertEqual(runner.memory_issues({"process": base}, self.config), [])
        for change in ({"affinity_escape": True}, {"affinity_verified": False},
                       {"cgroup_oom_events_delta": {"oom": 1, "oom_kill": 0}}):
            self.assertTrue(runner.memory_issues({"process": dict(base, **change)}, self.config))

    def fake_result(self, **updates):
        return dict({"stdout": b"{}\n", "stderr": b"", "exit_code": 0, "timed_out": False,
                     "elapsed_seconds": 0.01, "peak_rss_bytes": 1024, "rss_status": "OBSERVED",
                     "affinity_verified": True}, **updates)

    def run_fake(self, resume=False):
        return runner.execute("contract", ["fake", "--argument"], self.repo, self.output, self.source,
                              self.config, [1], "native", 10, resume)

    def test_resume_reuses_exact_success_without_running(self):
        with patch.object(runner.hpc, "run_process", return_value=self.fake_result()) as process:
            first = self.run_fake()
            second = self.run_fake(resume=True)
            self.assertEqual(first, second)
            self.assertEqual(process.call_count, 1)

    def test_failed_receipt_is_retained_and_not_retried(self):
        with patch.object(runner.hpc, "run_process", return_value=self.fake_result(exit_code=1)) as process:
            self.assertEqual(self.run_fake()["status"], "HOLD")
            self.assertEqual(self.run_fake(resume=True)["status"], "HOLD")
            self.assertEqual(process.call_count, 1)

    def test_resume_modified_raw_is_rejected(self):
        with patch.object(runner.hpc, "run_process", return_value=self.fake_result()):
            self.run_fake()
            (self.output / "runs/contract/stdout.log").write_text("tampered")
            with self.assertRaisesRegex(ValueError, "artifact"):
                self.run_fake(resume=True)

    def test_resume_cross_host_context_is_rejected(self):
        with patch.object(runner.hpc, "run_process", return_value=self.fake_result()):
            self.run_fake()
            self.config["_execution_identity_sha256"] = "b" * 64
            with self.assertRaisesRegex(ValueError, "request changed"):
                self.run_fake(resume=True)

    def test_resume_missing_artifact_manifest_is_rejected(self):
        with patch.object(runner.hpc, "run_process", return_value=self.fake_result()):
            self.run_fake()
            path = self.output / "runs/contract/RECEIPT.json"
            receipt = runner.read_json(path)
            receipt["artifacts"] = {}
            runner.write_json(path, receipt)
            with self.assertRaisesRegex(ValueError, "required artifacts"):
                self.run_fake(resume=True)

    def test_source_mutation_during_command_is_recorded_hold(self):
        def mutate(*args, **kwargs):
            (self.repo / "source.rs").write_text("changed during process")
            return self.fake_result()
        with patch.object(runner.hpc, "run_process", side_effect=mutate):
            receipt = self.run_fake()
            self.assertEqual(receipt["status"], "HOLD")
            self.assertNotEqual(receipt["source"], receipt["source_after"])

    def test_launch_failure_is_preserved_as_unknown_not_zero(self):
        with patch.object(runner.hpc, "run_process", side_effect=OSError("launcher missing")) as process:
            receipt = self.run_fake()
            self.assertEqual(receipt["status"], "HOLD")
            self.assertIsNone(receipt["process"]["exit_code"])
            self.assertIsNone(runner.process_elapsed_ns(receipt))
            self.assertIsNone(receipt["process"]["peak_rss_bytes"])
            self.assertEqual(self.run_fake(resume=True), receipt)
            self.assertEqual(process.call_count, 1)

    def test_stale_unreceipted_raw_file_cannot_be_inherited(self):
        raw = self.output / "WORK_PRECISION.json"
        raw.write_text('{"stale":true}')
        with patch.object(runner.hpc, "run_process") as process:
            with self.assertRaisesRegex(ValueError, "unreceipted raw"):
                runner.execute("fresh", ["fake"], self.repo, self.output, self.source,
                               self.config, [1], "bench", 10, False, raw_file=raw)
            process.assert_not_called()
        self.assertEqual(raw.read_text(), '{"stale":true}')

    def test_modified_benchmark_binary_cannot_be_used(self):
        binary = runner.produced_binaries("bench_build", self.output)[0]
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"built")
        rel = str(binary.relative_to(self.output))
        runtime = {"libraries": {"/lib/fake.so": "f" * 64}}
        runner.write_json(self.output / "runs/bench_build/RECEIPT.json", {
            "artifacts": {rel: runner.digest(b"built")}, "runtime_libraries": {rel: runtime}})
        with patch.object(runner, "runtime_dependencies", return_value=runtime):
            self.assertEqual(runner.executable_binding([str(binary)], self.output)["sha256"], runner.digest(b"built"))
        with patch.object(runner, "runtime_dependencies", return_value={"libraries": {"/lib/fake.so": "0" * 64}}):
            with self.assertRaisesRegex(ValueError, "shared libraries changed"):
                runner.executable_binding([str(binary)], self.output)
        binary.write_bytes(b"modified")
        with self.assertRaisesRegex(ValueError, "executable differs"):
            runner.executable_binding([str(binary)], self.output)

    def test_missing_dynamic_library_fails_closed(self):
        with patch.object(runner, "capture", return_value={"exit_code": 0, "stdout": "libfoo.so => not found\n"}):
            with self.assertRaises(ValueError):
                runner.runtime_dependencies(self.root / "fake")

    def test_incomplete_campaign_cannot_redefine_required_coverage(self):
        root = Path(runner.__file__).resolve().parents[1]
        config = runner.read_json(root / "research/production_reaudit_20261021/NCP_CONFIG.json")
        config["comparison_cases"].pop()
        with self.assertRaisesRegex(ValueError, "incomplete frozen"):
            runner.validate_config(config, root)

    def test_frozen_campaign_has_exact_unique_coverage(self):
        root = Path(runner.__file__).resolve().parents[1]
        config = runner.read_json(root / "research/production_reaudit_20261021/NCP_CONFIG.json")
        runner.validate_config(config, root)
        ids = runner.planned_case_ids(config)
        self.assertEqual(len(ids), 3221)
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(sum("warmup" in key for key in ids), 280)
        self.assertEqual(sum("repeat" in key for key in ids), 2940)
        self.assertEqual(ids[-1], "work_precision")

    def test_bound_clients_keep_distinct_mathematical_scopes(self):
        for name, (budget, scope, status, full) in runner.BOUND_CONTRACTS.items():
            case = {"suite": "bound-clients", "case": name, "original_budget": budget}
            row = dict(self.row, schema="vigilode.reaudit-bound-clients.v1", case=name,
                       original_budget=budget, achieved_bound=budget/2, scope=scope,
                       status=status, full_curve_certified=full)
            self.assertEqual(runner.numeric_row_issues(row, case, "protected", 1), [])
            row["scope"] = "GLOBAL_PRODUCTION"
            self.assertTrue(runner.numeric_row_issues(row, case, "protected", 1))

    def test_strict_json_rejects_nan(self):
        path = self.root / "invalid.json"
        path.write_text('{"number":NaN}')
        with self.assertRaises(ValueError):
            runner.read_json(path)

    def test_resume_file_escape_rejected(self):
        receipt = {"id": "contract", "artifacts": {"runs/contract/stdout.log": "x", "runs/contract/stderr.log": "x",
                                                        "../outside": "x"}}
        with self.assertRaises(ValueError):
            runner.verify_files(receipt, self.output)


if __name__ == "__main__":
    unittest.main()
