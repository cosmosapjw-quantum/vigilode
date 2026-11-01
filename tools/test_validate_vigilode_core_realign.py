#!/usr/bin/env python3
"""Same-author hostile tests of noncanonical science-first DAG and catalog."""
from __future__ import annotations
import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

S=importlib.util.spec_from_file_location(
    "vigilode_core_realign",
    Path(__file__).resolve().parent/"validate_vigilode_core_realign.py")
m=importlib.util.module_from_spec(S)
S.loader.exec_module(m)


class HostileAudit(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.orig=json.loads(m.SOURCE.read_text())
        cls.overlay=m.make()
        cls.rows=cls.overlay["nodes"]

    def test_immutability_original_byte_hash(self):
        self.assertEqual(m.sha(m.SOURCE),m.SOURCE_SHA256)
        self.assertEqual(len(self.orig["nodes"]),23)

    def test_ledger_not_rewritten_or_hidden_by_gate(self):
        self.assertEqual(self.overlay["historical_progress"]["closed_scoped"],7)
        self.assertEqual(self.overlay["historical_progress"]["pending_local"],15)
        self.assertEqual(self.overlay["historical_progress"]["authority_pending"],1)
        self.assertEqual(self.overlay["official_progress_claim"],"7/23_UNCHANGED")
        self.assertEqual(self.overlay["historical_successor_claim"],"0/12_UNCHANGED")

    def test_all_historical_release_dependencies_bit_semantically_preserved(self):
        self.assertEqual(
            [(n["id"],n["historical_status"],n["historical_release_dependencies"]) for n in self.rows],
            [(n["id"],n["status"],n["dependencies"]) for n in self.orig["nodes"]],
        )

    def test_research_edges_distinct_and_seven_only(self):
        diffs=[n["id"] for n in self.rows
               if n["research_start_dependencies"]!=n["historical_release_dependencies"]]
        self.assertEqual(diffs,[
            "L-HARDWARE","PR-N01","PR-N02","PR-N03","PR-N04","PR-N05","PR-N06"])
        m.ensure_sorted_dag(self.rows,"research_start_dependencies")
        m.ensure_sorted_dag(self.rows,"historical_release_dependencies")

    def test_cycle_is_refused_not_downgraded(self):
        bad=copy.deepcopy(self.rows)
        for n in bad:
            if n["id"]=="S-PRS":
                n["research_start_dependencies"]=["R-STRUCT"]
        with self.assertRaisesRegex(ValueError,"cycle"):
            m.ensure_sorted_dag(bad,"research_start_dependencies")

    def test_altered_original_source_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            fp=Path(tmp)/"fake.json"
            wrong=copy.deepcopy(self.orig)
            wrong["nodes"][0]["status"]="PASS"
            fp.write_text(json.dumps(wrong))
            with patch.object(m,"SOURCE",fp):
                with self.assertRaisesRegex(AssertionError,"ORIGINAL_23_NODE_PLAN_CHANGED"):
                    m.make()

    def test_no_production_shortcut(self):
        for n in self.rows:
            self.assertFalse(n["may_promote_production"])
        for key in ("no_merge","no_default_change","no_tolerance_relaxation","no_auto_next_node"):
            self.assertTrue(self.overlay[key])
        self.assertEqual(self.overlay["production"],"HOLD")

    def test_catalog_real_source_and_reference_guard(self):
        self.assertEqual(m.validate_catalog(),28)

    def test_no_applied_technical_sources_or_relabelled_backend(self):
        inv=json.loads(m.CATALOG.read_text())
        for row in inv["rows"]:
            text=row["source_path"].lower()
            self.assertNotIn("rei_",text)
            self.assertNotIn("bass",text)
            self.assertEqual(row["science_admission"],"not_promoted_by_this_audit")
        for ident in ("EXPRB2","EXPRB43","SABR5P","FUSED_POLYNOMIAL_PHI"):
            row=next(r for r in inv["rows"] if r["id"]==ident)
            self.assertNotEqual(row["maturity"],"full_ivp")


if __name__=="__main__":
    unittest.main(verbosity=2)
