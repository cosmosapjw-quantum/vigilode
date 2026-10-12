#!/usr/bin/env python3
"""Synthetic tests of tools/ht01_selection_check.py (research node HT01).

The valid document is built by tools/ht01_client_screen.py from the committed tree at HEAD (static scans and committed
files only), with a clean tree status injected; every other case mutates that document. Expected outcomes: INVALID
(exit 2) for malformed or unpinned evidence and the registered validity rules, FAIL for a recomputation, row, K1 or
decision mismatch, PASS for the untouched screen. The tools must be committed at HEAD (the checker-blob rule).
"""

from __future__ import annotations

import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import evidence_schema_v2 as evidence  # noqa: E402
import ht01_client_screen as screen  # noqa: E402
import ht01_selection_check as check  # noqa: E402

HEAD_CHECKER = subprocess.run(["git", "show", f"HEAD:{check.CHECKER_PATH}"], cwd=ROOT, capture_output=True,
                              check=True).stdout
BASE = screen.build_screen(ROOT, "HEAD", tree_status="")


def encode(doc) -> bytes:
    return json.dumps(doc, indent=1).encode()


def evaluate(doc=None, raw: bytes | None = None, checker: bytes = HEAD_CHECKER, status: str = ""):
    data = raw if raw is not None else encode(BASE if doc is None else doc)
    return check.evaluate(data, ROOT, checker, status)


def mutated(fn):
    doc = copy.deepcopy(BASE)
    fn(doc)
    return doc


def candidate(doc, cid):
    return next(c for c in doc["candidates"] if c["id"] == cid)


class Valid(unittest.TestCase):
    def test_untouched_screen_passes_with_registered_decision(self):
        verdict, report = evaluate()
        self.assertEqual(verdict, "PASS", report["gates"])
        self.assertEqual(report["decision"], BASE["decision"]["outcome"])
        self.assertEqual([r["id"] for r in report["candidates"]], check.IDS)


class Invalid(unittest.TestCase):
    def assertInvalid(self, *args, **kwargs):
        with self.assertRaises(evidence.ValidationError):
            evaluate(*args, **kwargs)

    def test_duplicate_key(self):
        raw = encode(BASE)
        self.assertInvalid(raw=raw.replace(b"{", b'{"node": "HT01",', 1))

    def test_nan(self):
        raw = encode(BASE).replace(b'"f_min": 0.348', b'"f_min": NaN', 1)
        self.assertIn(b"NaN", raw)
        self.assertInvalid(raw=raw)

    def test_extra_key(self):
        self.assertInvalid(mutated(lambda d: candidate(d, "K5")["S1"].update(extra=1)))

    def test_missing_key(self):
        self.assertInvalid(mutated(lambda d: candidate(d, "K6")["S2"].pop("rows")))

    def test_bool_as_int(self):
        self.assertInvalid(mutated(lambda d: d["provenance"].update(solver_runs=False)))

    def test_int_as_float(self):
        self.assertInvalid(mutated(lambda d: d["constants"].update(rhs_multiplier_r=5.0)))

    def test_changed_threshold(self):
        self.assertInvalid(mutated(lambda d: d["constants"].update(f_min=0.35)))

    def test_arm_list_differs_from_closed_list(self):
        self.assertInvalid(mutated(lambda d: candidate(d, "K5")["S3"]["applicable_arms"].pop()))

    def test_arm_outside_closed_list(self):
        def fn(d):
            candidate(d, "K6")["S3"]["per_case"][0]["rtols"][0]["ranking"][0]["config"] = "rodas5p-routed"
        self.assertInvalid(mutated(fn))

    def test_unpinned_input(self):
        def fn(d):
            item = next(i for i in d["inputs"] if i["path"] == check.PP07)
            item["sha256"] = "0" * 64
        self.assertInvalid(mutated(fn))

    def test_input_missing_from_pins(self):
        def fn(d):
            d["inputs"] = [i for i in d["inputs"] if i["role"] != "profile"]
        self.assertInvalid(mutated(fn))

    def test_rust_source_not_pinned(self):
        self.assertInvalid(mutated(lambda d: d["rust_sources"][0].__setitem__(1, "1" * 64)))

    def test_dirty_screen_tree(self):
        self.assertInvalid(mutated(lambda d: d["provenance"].update(tree_status=" M tools/x.py\n")))

    def test_dirty_checker_tree(self):
        self.assertInvalid(status=" M crates/rodas5p-core/src/lib.rs")

    def test_running_checker_differs(self):
        self.assertInvalid(checker=HEAD_CHECKER + b"\n# edited\n")

    def test_recorded_checker_blob_differs(self):
        self.assertInvalid(mutated(lambda d: d["provenance"].update(checker_blob="0" * 40)))

    def test_unknown_commit(self):
        self.assertInvalid(mutated(lambda d: d["provenance"].update(source_commit="f" * 40)))


class Fail(unittest.TestCase):
    def assertFails(self, doc, item):
        verdict, report = evaluate(doc)
        self.assertEqual(verdict, "FAIL")
        self.assertFalse(report["gates"][item]["pass"], item)
        return report

    def test_tampered_f_min(self):
        self.assertFails(mutated(lambda d: candidate(d, "K5")["S1"].update(f_min=0.5)), "2_recomputed")

    def test_tampered_share(self):
        self.assertFails(mutated(lambda d: candidate(d, "K6")["S1"]["f_values"][0].update(share_file=0.9)),
                         "2_recomputed")

    def test_tampered_flag(self):
        self.assertFails(mutated(lambda d: candidate(d, "K5")["meets"].update(S4=True)), "2_recomputed")

    def test_tampered_comparator(self):
        def fn(d):
            r = candidate(d, "K6")["S3"]["per_case"][-1]["rtols"][0]
            r["comparator"] = dict(r["comparator"], config="rodas5p-fast", arm="dense v2")
        self.assertFails(mutated(fn), "2_recomputed")

    def test_tampered_decision(self):
        def fn(d):
            d["decision"] = {"outcome": "SELECTED(K5)", "selected": "K5", "meeting_all": ["K5"], "leads": []}
        self.assertFails(mutated(fn), "4_decision_rule_and_contract")

    def test_missing_candidate(self):
        self.assertFails(mutated(lambda d: d["candidates"].pop()), "1_rows")

    def test_reordered_candidates(self):
        self.assertFails(mutated(lambda d: d["candidates"].reverse()), "1_rows")

    def test_k1_row(self):
        report = self.assertFails(mutated(lambda d: candidate(d, "K1")["S2"].update(pp07="PP07 reopened")),
                                  "3_k1_and_no_artificial_workload")
        self.assertFalse(report["gates"]["2_recomputed"]["pass"])

    def test_non_git_command(self):
        self.assertFails(mutated(lambda d: d["provenance"]["executed_commands"].append(["cargo", "test"])),
                         "3_k1_and_no_artificial_workload")

    def test_artificial_workload(self):
        self.assertFails(mutated(lambda d: d["reported"]["artificial_workloads"].append("65-shift")),
                         "3_k1_and_no_artificial_workload")


class Rule(unittest.TestCase):
    @staticmethod
    def row(cid, met, f=None, lead=False):
        return {"id": cid, "meets": {"S1": met, "S2": met, "S3": met, "S4": met}, "S1": {"f_min": f}, "lead": lead}

    def test_none_abstains(self):
        self.assertEqual(check.rule([self.row("K1", False), self.row("K2", False)])["outcome"], "ABSTAIN")

    def test_exactly_one_selected(self):
        self.assertEqual(check.rule([self.row("K1", False), self.row("K2", True)])["outcome"], "SELECTED(K2)")

    def test_largest_minimum_f(self):
        rows = [self.row("K5", True, 0.40), self.row("K6", True, 0.50)]
        self.assertEqual(check.rule(rows)["outcome"], "SELECTED(K6)")

    def test_unmeasured_ranks_below_measured(self):
        rows = [self.row("K2", True, None), self.row("K6", True, 0.36)]
        self.assertEqual(check.rule(rows)["outcome"], "SELECTED(K6)")

    def test_tie_abstains(self):
        rows = [self.row("K2", True, None), self.row("K3", True, None)]
        self.assertEqual(check.rule(rows)["outcome"], "ABSTAIN")

    def test_screen_rule_agrees(self):
        for rows in ([self.row("K2", True)], [self.row("K5", True, 0.40), self.row("K6", True, 0.50)],
                     [self.row("K2", True, None), self.row("K3", True, None)]):
            shaped = [dict(r, all=all(r["meets"].values())) for r in rows]
            self.assertEqual(screen.decide(shaped)["outcome"], check.rule(rows)["outcome"])

    def test_leads(self):
        rows = [self.row("K5", False, 0.40, lead=True), self.row("K6", False, 0.1)]
        self.assertEqual(check.rule(rows)["leads"], ["K5"])


class Cli(unittest.TestCase):
    def test_garbage_screen_is_invalid_exit_2(self):
        with tempfile.TemporaryDirectory() as tmp:
            bad, out = Path(tmp) / "SCREEN.json", Path(tmp) / "RESULTS.json"
            bad.write_text('{"a": 1, "a": 2}')
            self.assertEqual(check.run(bad, out, ROOT), 2)
            self.assertEqual(json.loads(out.read_text())["verdict"], "INVALID")

    def test_output_is_immutable(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "RESULTS.json"
            out.write_text("{}")
            with self.assertRaises(SystemExit):
                check.run(out, out, ROOT)


if __name__ == "__main__":
    unittest.main(verbosity=1)
