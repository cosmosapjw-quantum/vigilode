import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("exporter", Path(__file__).with_name("export_local_measurements.py"))
exporter = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(exporter)


class ExportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base / "runs"
        self.root.mkdir()

    def fixture(self, name="one", status="COMPLETED", result=None):
        folder = self.root / name
        folder.mkdir()
        if result is None and status == "COMPLETED":
            result = {"kind": "result", "case": 15, "method": "dopri5", "parallel_layer": "serial",
                      "threads": 1, "elapsed_seconds": 0.1, "actual_error": 1e-8,
                      "error_limit": 1e-4, "final_time": 1.0, "final_state": [1.0, 2.0],
                      "scientific_status": "PASS_SCOPED", "solver_success": True,
                      "counters": {"rhs_callbacks": 20}, "timing_scope": "synthetic solve"}
        stdout = (json.dumps(result) + "\n").encode() if result else b""
        (folder / "stdout.jsonl").write_bytes(stdout)
        (folder / "stderr.log").write_bytes(b"")
        (folder / "resource.txt").write_bytes(b"resource fixture\n")
        raw = {"execution_status": status, "finished_utc": "2026-10-25T00:00:00Z",
               "exit_code": 0 if status == "COMPLETED" else 124,
               "elapsed_process_seconds": .2, "result": result,
               "task": {"id": name, "case": 15, "method": "dopri5", "parallel_layer": "serial", "threads": 1},
               "source": {"commit": "a" * 40, "clean": True},
               "binary_sha256": "b" * 64}
        for key, filename in (("stdout", "stdout.jsonl"), ("stderr", "stderr.log"), ("resource", "resource.txt")):
            raw[key + "_sha256"] = hashlib.sha256((folder / filename).read_bytes()).hexdigest()
        (folder / "RECEIPT.json").write_text(json.dumps(raw))
        return folder, raw

    def test_verified_scalars_vectors_omitted_and_input_unchanged(self):
        folder, _ = self.fixture()
        before = {p.name: p.read_bytes() for p in folder.iterdir()}
        summary = exporter.export(self.root, self.base / "export")
        row = json.loads((self.base / "export/MEASUREMENTS.jsonl").read_text())
        self.assertEqual(summary["evidence_status_counts"], {"RAW_VERIFIED": 1})
        self.assertTrue(row["timing_reanalysis_candidate"])
        self.assertEqual(row["result_scalars"]["counters.rhs_callbacks"], 20)
        self.assertNotIn("final_state", row["result_scalars"])
        self.assertEqual(before, {p.name: p.read_bytes() for p in folder.iterdir()})

    def test_tampered_stream_not_eligible(self):
        folder, _ = self.fixture()
        (folder / "stdout.jsonl").write_text('{}\n')
        row = exporter.export_receipt(folder / "RECEIPT.json", self.root)
        self.assertIn("HASH_MISMATCH_STDOUT", row["issues"])
        self.assertIn("STORED_RAW_RESULT_MISMATCH", row["issues"])
        self.assertFalse(row["timing_reanalysis_candidate"])

    def test_missing_stream_is_distinct(self):
        folder, _ = self.fixture()
        (folder / "stderr.log").unlink()
        row = exporter.export_receipt(folder / "RECEIPT.json", self.root)
        self.assertIn("MISSING_STDERR", row["issues"])

    def test_header_timing_scope_and_resource_costs_preserved(self):
        folder, raw = self.fixture()
        header = {"kind": "header", "timing_scope": "whole solve incl workspace", "rtol": 1e-6,
                  "model_contract": {"initial_state": [1, 2], "dimension": 2}}
        text = (json.dumps(header) + "\n" + json.dumps(raw["result"]) + "\n").encode()
        (folder / "stdout.jsonl").write_bytes(text)
        raw["stdout_sha256"] = hashlib.sha256(text).hexdigest()
        resource = b"\tUser time (seconds): 1.20\n\tSystem time (seconds): 0.30\n\tMaximum resident set size (kbytes): 4000\n"
        (folder / "resource.txt").write_bytes(resource)
        raw["resource_sha256"] = hashlib.sha256(resource).hexdigest()
        (folder / "RECEIPT.json").write_text(json.dumps(raw))
        row = exporter.export_receipt(folder / "RECEIPT.json", self.root)
        self.assertEqual(row["header_scalars"]["timing_scope"], "whole solve incl workspace")
        self.assertEqual(row["header_scalars"]["model_contract.dimension"], 2)
        self.assertNotIn("model_contract.initial_state", row["header_scalars"])
        self.assertEqual(row["resource_scalars"]["System time (seconds)"], .3)

    def test_timeout_and_unfinished_attempt_preserved(self):
        self.fixture(status="TIMEOUT")
        active = self.root / "active"
        active.mkdir()
        (active / "START.json").write_text('{"started":true}')
        summary = exporter.export(self.root, self.base / "export")
        self.assertEqual(summary["receipt_files"], 1)
        self.assertEqual(summary["evidence_status_counts"], {"RAW_VERIFIED": 1})
        self.assertEqual(summary["incomplete_attempts"][0]["status"], "INCOMPLETE_ACTIVE_OR_INTERRUPTED")

    def test_nonfinite_json_is_rejected(self):
        folder, raw = self.fixture()
        raw["result"]["final_time"] = float("nan")
        (folder / "RECEIPT.json").write_text(json.dumps(raw))
        row = exporter.export_receipt(folder / "RECEIPT.json", self.root)
        self.assertEqual(row["evidence_status"], "READ_OR_FORMAT_ERROR")
        self.assertFalse(row["timing_reanalysis_candidate"])

    def test_invalid_time_and_mismatched_task_rejected(self):
        folder, raw = self.fixture()
        raw["result"]["elapsed_seconds"] = -1
        raw["result"]["method"] = "wrong"
        text = (json.dumps(raw["result"]) + "\n").encode()
        (folder / "stdout.jsonl").write_bytes(text)
        raw["stdout_sha256"] = hashlib.sha256(text).hexdigest()
        (folder / "RECEIPT.json").write_text(json.dumps(raw))
        row = exporter.export_receipt(folder / "RECEIPT.json", self.root)
        self.assertIn("NONPOSITIVE_ELAPSED_SECONDS", row["issues"])
        self.assertIn("CONTRACT_MISMATCH_METHOD", row["issues"])

    def test_index_distinguishes_missing_and_different_receipt(self):
        folder, _ = self.fixture()
        ref = self.base / "index.json"
        ref.write_text(json.dumps({"rows": [
            {"task_id": "one", "receipt_sha256": hashlib.sha256((folder / "RECEIPT.json").read_bytes()).hexdigest()},
            {"task_id": "one", "receipt_sha256": "0" * 64},
            {"task_id": "absent", "receipt_sha256": "1" * 64}]}))
        summary = exporter.export(self.root, self.base / "export", ref)
        self.assertEqual(summary["task_index"]["counts"], {"EXACT_RECEIPT_BYTES_FOUND": 1,
            "TASK_FOUND_DIFFERENT_RECEIPT_BYTES": 1, "RAW_RECEIPT_NOT_FOUND": 1})

    def test_wrapper_and_raw_share_evidence_key(self):
        folder, raw = self.fixture()
        outer = {"raw_receipt": raw, "case": 15, "method": "dopri5", "warmup": False,
                 "assessment": {"status": "PASS_SCOPED", "actual_error": 1e-8, "error_limit": 1e-4,
                                "reference_uncertainty_bound": 1e-12}}
        (folder / "COMPARISON_RECEIPT.json").write_text(json.dumps(outer))
        summary = exporter.export(self.root, self.base / "export")
        self.assertEqual(summary["receipt_files"], 2)
        self.assertEqual(summary["raw_evidence_keys"], 1)

    def test_instrumented_failed_accuracy_and_warmup_not_candidates(self):
        folder, raw = self.fixture()
        for key, value in (("instrumented", True), ("scientific_status", "ACCURACY_FAILED")):
            changed = json.loads(json.dumps(raw))
            changed["result"][key] = value
            text = (json.dumps(changed["result"]) + "\n").encode()
            (folder / "stdout.jsonl").write_bytes(text)
            changed["stdout_sha256"] = hashlib.sha256(text).hexdigest()
            (folder / "RECEIPT.json").write_text(json.dumps(changed))
            self.assertFalse(exporter.export_receipt(folder / "RECEIPT.json", self.root)["timing_reanalysis_candidate"])

    def test_output_must_be_new_and_external(self):
        self.fixture()
        with self.assertRaises(ValueError):
            exporter.export(self.root, self.root / "export")
        self.assertFalse((self.root / "export").exists())
        (self.base / "existing").mkdir()
        with self.assertRaises(FileExistsError):
            exporter.export(self.root, self.base / "existing")

    def test_escape_symlink_rejected(self):
        folder, raw = self.fixture()
        outside = self.base / "outside"
        outside.write_text("")
        (folder / "stderr.log").unlink()
        (folder / "stderr.log").symlink_to(outside)
        row = exporter.export_receipt(folder / "RECEIPT.json", self.root)
        self.assertEqual(row["evidence_status"], "READ_OR_FORMAT_ERROR")

    def test_receipt_disappears_at_final_read_never_eligible(self):
        folder, _ = self.fixture()
        read = exporter.bounded_read
        calls = 0
        def disappearing(path):
            nonlocal calls
            if path.name == "RECEIPT.json":
                calls += 1
                if calls == 2:
                    raise FileNotFoundError("removed during export")
            return read(path)
        with patch.object(exporter, "bounded_read", side_effect=disappearing):
            row = exporter.export_receipt(folder / "RECEIPT.json", self.root)
        self.assertEqual(row["evidence_status"], "READ_OR_FORMAT_ERROR")
        self.assertFalse(row["timing_reanalysis_candidate"])

    def test_index_requires_identity_and_hash_together(self):
        folder, _ = self.fixture()
        ref = self.base / "index.json"
        digest = hashlib.sha256((folder / "RECEIPT.json").read_bytes()).hexdigest()
        ref.write_text(json.dumps({"rows": [{"task_id": "wrong", "receipt_sha256": digest}]}))
        summary = exporter.export(self.root, self.base / "export", ref)
        self.assertEqual(summary["task_index"]["counts"], {"RECEIPT_TASK_ID_MISMATCH": 1})
        ref.write_text('{"rows":[{"task_id":null,"receipt_sha256":null}]}')
        with self.assertRaises(ValueError):
            exporter.export(self.root, self.base / "other", ref)
        self.assertFalse((self.base / "other").exists())

    def test_huge_integer_does_not_abort_export(self):
        folder, raw = self.fixture()
        raw["result"]["elapsed_seconds"] = 10 ** 400
        text = (json.dumps(raw["result"]) + "\n").encode()
        (folder / "stdout.jsonl").write_bytes(text)
        raw["stdout_sha256"] = hashlib.sha256(text).hexdigest()
        (folder / "RECEIPT.json").write_text(json.dumps(raw))
        row = exporter.export_receipt(folder / "RECEIPT.json", self.root)
        self.assertIn("INVALID_ELAPSED_SECONDS", row["issues"])
        self.assertFalse(row["timing_reanalysis_candidate"])


if __name__ == "__main__":
    unittest.main()
