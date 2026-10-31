#!/usr/bin/env python3
"""Pure structural regression and negative tests, not science verification."""
from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from check_mission_dag import validate


DAG = json.loads((Path(__file__).resolve().parents[1] / "governance/mission_dag.json").read_text())


class MissionTests(unittest.TestCase):
    def test_current_overlay_passes(self):
        self.assertEqual(validate(DAG), [])

    def test_duplicate_id_fails(self):
        doc = copy.deepcopy(DAG)
        doc["nodes"][1]["id"] = doc["nodes"][0]["id"]
        self.assertTrue(any("duplicate" in x for x in validate(doc)))

    def test_dangling_dependency_fails(self):
        doc = copy.deepcopy(DAG)
        doc["nodes"][1]["depends_on"] = ["MISSING"]
        self.assertTrue(any("missing dependency" in x for x in validate(doc)))

    def test_cycle_fails(self):
        doc = copy.deepcopy(DAG)
        doc["nodes"][0]["depends_on"] = ["M5"]
        self.assertTrue(any("cycle" in x for x in validate(doc)))

    def test_malformed_dependency_fails_closed_without_exception(self):
        doc = copy.deepcopy(DAG)
        doc["nodes"][1]["depends_on"] = 12
        self.assertTrue(any("depends_on must" in x for x in validate(doc)))

    def test_false_admission_fails(self):
        doc = copy.deepcopy(DAG)
        doc["nodes"][0]["state"] = "admitted"
        self.assertTrue(any("admission requires external" in x for x in validate(doc)))

    def test_counter_inflation_fails(self):
        doc = copy.deepcopy(DAG)
        doc["canonical_dag_delta_from_this_change"] = 1
        self.assertTrue(any("canonical DAG delta" in x for x in validate(doc)))

    def test_authority_ambiguity_fails(self):
        doc = copy.deepcopy(DAG)
        doc["canonical_authority"].pop("rei_main_state")
        self.assertTrue(any("authority" in x for x in validate(doc)))

    def test_promotion_or_autonext_fails(self):
        doc = copy.deepcopy(DAG)
        doc["work_stream_policy"]["production"] = "READY"
        doc["work_stream_policy"]["auto_next_node"] = True
        errs = validate(doc)
        self.assertTrue(any("production" in x for x in errs))
        self.assertTrue(any("auto_next_node" in x for x in errs))

    def test_unadmitted_dependency_not_ready(self):
        doc = copy.deepcopy(DAG)
        doc["nodes"][1]["state"] = "ready"
        self.assertTrue(any("ready despite" in x for x in validate(doc)))

    def test_bad_stokes_owner_fails(self):
        doc = copy.deepcopy(DAG)
        doc["nodes"][0]["owner"] = "vigilode_assumed_cmb_owner"
        self.assertTrue(any("unknown owner" in x for x in validate(doc)))

    def test_no_dag_fails(self):
        self.assertTrue(validate({"schema_version": 1}))


if __name__ == "__main__":
    unittest.main()
