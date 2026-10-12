#!/usr/bin/env python3
"""Synthetic tests of tools/ht02_parallel_check.py (research node HT02, Branch A).

The valid input is HT01's committed RESULTS.json; the other cases mutate it (re-pinned where the test is about a
later rule, so that each case isolates one rule). INVALID raises evidence.ValidationError (exit 2 in the CLI).
"""

from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import evidence_schema_v2 as evidence  # noqa: E402
import ht02_parallel_check as check  # noqa: E402

RAW = (ROOT / check.HT01_RESULTS).read_bytes()
BASE = json.loads(RAW)
CHECKER = b"# checker bytes\n"
CLEAN = {"source_commit": "0" * 40, "tree_status": "", "checker_blob_at_head": check.blob_id(CHECKER)}


def evaluate(raw: bytes = RAW, prov=CLEAN, repin: bool = False):
    pin = hashlib.sha256(raw).hexdigest() if repin else check.HT01_RESULTS_SHA256
    return check.evaluate(raw, CHECKER, prov, pin)


def mutated(fn) -> bytes:
    doc = copy.deepcopy(BASE)
    fn(doc)
    return json.dumps(doc, indent=1).encode()


class BranchA(unittest.TestCase):
    def test_committed_ht01_results_is_the_pinned_file(self):
        self.assertEqual(hashlib.sha256(RAW).hexdigest(), check.HT01_RESULTS_SHA256)

    def test_abstain_not_run(self):
        found = evaluate()
        self.assertEqual(found["ht01"]["decision"], "ABSTAIN")
        self.assertEqual(sorted(found["failing_criteria"]), check.IDS)
        for cid, row in found["failing_criteria"].items():
            self.assertTrue(row["criteria_not_met"], cid)
            self.assertIn("S1", row["criteria_not_met"])

    def test_cli_writes_registered_outcome(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "RESULTS.json"
            prov = check.provenance(ROOT, [])
            code = check.run(ROOT / check.HT01_RESULTS, out, True, ROOT)
            doc = json.loads(out.read_text())
            if prov["tree_status"] == "" and prov["checker_blob_at_head"] == check.blob_id(
                    (TOOLS / "ht02_parallel_check.py").read_bytes()):
                self.assertEqual(code, 0)
                self.assertEqual((doc["decision"], doc["verdict"], doc["runs"], doc["wall_time_speedup"]),
                                 ("ABSTAIN_NOT_RUN", "INCONCLUSIVE", 0, None))
            else:  # an uncommitted checker or a dirty tree is INVALID by design
                self.assertEqual((code, doc["verdict"]), (2, "INVALID"))


class Invalid(unittest.TestCase):
    def assertInvalid(self, *args, **kwargs):
        with self.assertRaises(evidence.ValidationError):
            evaluate(*args, **kwargs)

    def test_unpinned_file(self):
        self.assertInvalid(raw=RAW + b"\n")

    def test_duplicate_key(self):
        self.assertInvalid(raw=RAW.replace(b"{", b'{"verdict": "PASS",', 1), repin=True)

    def test_selected_decision_needs_branch_b(self):
        self.assertInvalid(raw=mutated(lambda d: d.update(decision="SELECTED(K2)")), repin=True)

    def test_ht01_not_pass(self):
        self.assertInvalid(raw=mutated(lambda d: d.update(verdict="FAIL")), repin=True)

    def test_wrong_candidates(self):
        self.assertInvalid(raw=mutated(lambda d: d["candidates"].pop()), repin=True)

    def test_row_key_set(self):
        self.assertInvalid(raw=mutated(lambda d: d["candidates"][0].pop("reasons")), repin=True)

    def test_criterion_not_bool(self):
        self.assertInvalid(raw=mutated(lambda d: d["candidates"][0].update(S1=0)), repin=True)

    def test_dirty_tree(self):
        self.assertInvalid(prov=dict(CLEAN, tree_status=" M tools/x.py"))

    def test_checker_not_committed(self):
        self.assertInvalid(prov=dict(CLEAN, checker_blob_at_head=None))

    def test_branch_b_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "RESULTS.json"
            self.assertEqual(check.run(ROOT / check.HT01_RESULTS, out, False, ROOT), 2)
            self.assertEqual(json.loads(out.read_text())["verdict"], "INVALID")

    def test_output_is_immutable(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "RESULTS.json"
            out.write_text("{}")
            with self.assertRaises(SystemExit):
                check.run(ROOT / check.HT01_RESULTS, out, True, ROOT)


if __name__ == "__main__":
    unittest.main(verbosity=1)
