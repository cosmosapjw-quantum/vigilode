#!/usr/bin/env python3
"""Hostile A07 checks use retained measured rows; no solver campaign is rerun."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location("a07",ROOT/"tools/all_dag_benchmark.py")
a07=importlib.util.module_from_spec(spec)
spec.loader.exec_module(a07)
RAW=ROOT/"research/cloud_completion_20261018/evidence/local_codex_rerun_20261018/bench/structured_comparison.stdout"


class A07Contracts(unittest.TestCase):
    def setUp(self):
        self.entries=a07.load_rows(RAW)
        self.manifest={"processes":[{"id":0,"modes":32}],"repeats":5,
          "source_binding":"RETAINED_HISTORICAL_ROWS_NO_NEW_BUILD_BINDING"}

    def test_source_identity_binds_actual_coefficients_and_build_script(self):
        identity=a07.source_identity()
        for name in ("fixtures/rodas5p_coefficients_snapshot.json",
                     "crates/rodas5p-integrators/build.rs"):
            self.assertEqual(identity[name],a07.digest(ROOT/name))

    def test_retained_rows_pass_without_promoting_historical_binding(self):
        result=a07.summarize(self.entries,self.manifest)
        self.assertTrue(result["schema_valid"])
        self.assertEqual(result["rows_retained"],30)
        self.assertTrue(all(g["matched_achieved_accuracy"] for g in result["groups"]))
        self.assertTrue(all(not g["matched_accuracy_comparison_admitted"] for g in result["groups"]))
        self.assertTrue(all(g["paired_dense_over_structured_timing"]["insufficient_independent_processes"] for g in result["groups"]))
        self.assertEqual(result["speed_promotion"],"HOLD")

    def test_counterexample_same_requested_tolerance_is_not_matched_error(self):
        for e in self.entries:
            r=e["raw"]
            if r["arm"]=="structured": r["max_abs_reference_error"]*=2
        result=a07.summarize(self.entries,self.manifest)
        self.assertFalse(any(g["matched_achieved_accuracy"] for g in result["groups"]))

    def test_missing_and_duplicate_arm_block_comparison(self):
        missing=a07.summarize(self.entries[1:],self.manifest)
        duplicate=a07.summarize(self.entries+[self.entries[0]],self.manifest)
        self.assertFalse(missing["schema_valid"])
        self.assertEqual(len(missing["missing_identities"]),1)
        self.assertFalse(duplicate["schema_valid"])
        self.assertEqual(len(duplicate["duplicate_identities"]),1)

    def test_failed_arm_is_retained(self):
        self.entries[0]["raw"]["success"]=False
        self.entries[0]["raw"]["empirical_accuracy_gate_pass"]=False
        self.entries[0]["raw"]["message"]="injected solver failure"
        result=a07.summarize(self.entries,self.manifest)
        self.assertEqual(result["rows_retained"],30)
        self.assertEqual(sum(g["arms"]["structured"]["failed"] for g in result["groups"]),1)
        self.assertTrue(any(not g["matched_achieved_accuracy"] for g in result["groups"]))

    def test_allocator_false_zero_and_false_gate_rejected(self):
        row=copy.deepcopy(self.entries[0]["raw"])
        row["allocation_calibration_valid"]=False
        row["allocation_calibration_library_clone_calls"]=0
        row["allocation_calls"]=0
        self.assertIn("uncalibrated_allocation_claim",a07.validate_row(row))
        row=copy.deepcopy(self.entries[0]["raw"])
        row["empirical_accuracy_gate_upper"]=0
        self.assertIn("false_accuracy_sum",a07.validate_row(row))

    def test_nan_and_truncation_preserved_as_invalid(self):
        with tempfile.TemporaryDirectory() as td:
            path=Path(td)/"raw.jsonl"
            path.write_text('{"elapsed_ns":NaN}\n[1,2]\n{"broken"\n')
            rows=a07.load_rows(path)
            self.assertEqual(len(rows),3)
            self.assertTrue(all(r["errors"] for r in rows))

    def test_generation_roundtrip_no_report_data_discrepancy(self):
        result=a07.summarize(self.entries,self.manifest)
        json_roundtrip=json.loads(json.dumps(result,allow_nan=False))
        self.assertEqual(a07.markdown(result),a07.markdown(json_roundtrip))
        self.assertIn("Speed promotion: **HOLD**",a07.markdown(result))

    def test_bootstrap_uses_process_units(self):
        extra=[]
        for process in (1,2):
            for e in self.entries:
                item=copy.deepcopy(e)
                item["process_id"]=process
                extra.append(item)
        manifest=copy.deepcopy(self.manifest)
        manifest["processes"]=[{"id":i,"modes":32} for i in range(3)]
        result=a07.summarize(self.entries+extra,manifest)
        self.assertTrue(result["schema_valid"])
        self.assertTrue(all(g["paired_dense_over_structured_timing"]["unit_count"]==3 for g in result["groups"]))


if __name__=="__main__": unittest.main()
