#!/usr/bin/env python3
"""Tests of tools/triage/triage_profile.py and tools/triage/triage_alloc.py with fake executables.

The fake valgrind (tools/test_triage_runner.py::FAKE_VALGRIND) runs the fake `rodas5p triage-profile-run` and
writes a callgrind/cachegrind file whose total is Ir(K) = 4000 for K = 0 and 5000 + 1300 K otherwise, so
ir_warm = Ir(2) - Ir(1) = 1300 and ir_cold = Ir(1) - Ir(0) = 2300; with FAKE_NONDET the second K = 1 process
(`1b`) reports one more instruction.

Run: python3 -m pytest tools/test_triage_profile.py -q
"""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
if str(ROOT / "tools") not in sys.path:
    sys.path.insert(0, str(ROOT / "tools"))
from test_triage_runner import FakeEnv, read_rows  # noqa: E402


class ProfileTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="triage_profile_")
        self.fake = FakeEnv(Path(self._tmp.name))
        self.out = self.fake.root / "PROFILE.jsonl"
        self.state = self.fake.root / "valgrind.state"

    def tearDown(self):
        self._tmp.cleanup()

    def profile(self, *extra: str, env: dict | None = None):
        return self.fake.run("triage_profile.py", "--binary", str(self.fake.binary), "--list", str(self.fake.list),
                             "--corpus", str(self.fake.corpus), "--campaign", "ct-fake", "--output", str(self.out),
                             "--scratch", str(self.fake.root / "scratch"), "--valgrind", str(self.fake.valgrind),
                             "--arm", "ok-arm", *extra, extra_env=dict({"FAKE_STATE": str(self.state)}, **(env or {})))

    def test_ir_arithmetic_and_determinism(self):
        proc = self.profile()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        rows = read_rows(self.out)
        # profile subset: rtol 1e-6, dimension <= 100 (robertson only), endpoint, h0 = 1
        self.assertEqual([(r["problem"], r["rtol"], r["h0_scale"]) for r in rows], [("robertson", 1e-6, 1.0)])
        row = rows[0]
        self.assertEqual(row["ir"], {"0": 4000, "1": 6300, "2": 7600, "1b": 6300})
        self.assertEqual((row["ir_warm"], row["ir_cold"]), (1300, 2300))
        self.assertIs(row["deterministic"], True)
        self.assertEqual(row["status"], "ok")
        self.assertEqual(row["summary"]["attempts"], 12)
        self.assertEqual(row["binary"]["profile"], "bench")
        self.assertEqual(row["valgrind"], "valgrind-3.99.fake")
        self.assertIsNone(row["cachegrind"])
        self.assertEqual(list((self.fake.root / "scratch").iterdir()), [])  # output files removed by default

    def test_nondeterministic_flag(self):
        proc = self.profile(env={"FAKE_NONDET": "1"})
        self.assertEqual(proc.returncode, 0, proc.stderr)
        row = read_rows(self.out)[0]
        self.assertEqual(row["ir"]["1b"], row["ir"]["1"] + 1)
        self.assertIs(row["deterministic"], False)
        self.assertEqual(row["ir_warm"], 1300)

    def test_cachegrind_events_two_minus_one(self):
        proc = self.profile("--cachegrind")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        cg = read_rows(self.out)[0]["cachegrind"]
        self.assertEqual(cg["status"], "ok")
        self.assertEqual(cg["warm"], {"Ir": 1300, "I1mr": 1, "D1mr": 10, "DLmr": 2})

    def test_refuses_non_bench_binary(self):
        proc = self.profile(env={"FAKE_PROFILE": "release"})
        self.assertEqual(proc.returncode, 2)
        self.assertIn("not 'bench'", proc.stderr)
        self.assertFalse(self.out.exists())

    def test_valgrind_failure_is_a_typed_null_not_zero(self):
        proc = self.profile(env={"FAKE_VALGRIND_FAIL": "1"})
        self.assertEqual(proc.returncode, 0, proc.stderr)
        row = read_rows(self.out)[0]
        self.assertEqual(row["status"], "failed")
        self.assertIn("valgrind: fatal error", row["reason"])
        self.assertIsNone(row["ir_warm"])
        self.assertIsNone(row["deterministic"])

    def test_resume_skips_present_cells(self):
        self.assertEqual(self.profile().returncode, 0)
        before = self.out.read_bytes()
        calls = len(self.fake.calls("triage-profile-run"))
        self.assertEqual(self.profile().returncode, 0)
        self.assertEqual(self.out.read_bytes(), before)
        self.assertEqual(len(self.fake.calls("triage-profile-run")), calls)


class AllocTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="triage_alloc_")
        self.fake = FakeEnv(Path(self._tmp.name))
        self.out = self.fake.root / "ALLOCATIONS.jsonl"

    def tearDown(self):
        self._tmp.cleanup()

    def alloc(self, arm: str, env: dict | None = None):
        return self.fake.run("triage_alloc.py", "--binary", str(self.fake.binary), "--list", str(self.fake.list),
                             "--corpus", str(self.fake.corpus), "--campaign", "ct-fake", "--output", str(self.out),
                             "--arm", arm, "--warmups", "2", "--windows", "3", extra_env=env)

    def test_warm_is_max_over_raw_windows(self):
        proc = self.alloc("ok-arm", {"FAKE_WINDOWS": "0,3,1"})
        self.assertEqual(proc.returncode, 0, proc.stderr)
        row = read_rows(self.out)[0]
        self.assertEqual(row["status"], "ok")
        self.assertEqual([w["allocations"] for w in row["windows"]], [0, 3, 1])
        self.assertEqual(row["warm"], {"allocations": 3, "deallocations": 3, "allocated_bytes": 48,
                                       "peak_live_bytes": 0})
        self.assertEqual(row["cold"]["allocations"], 40)

    def test_window_count_mismatch_is_invalid(self):
        proc = self.alloc("ok-arm", {"FAKE_WINDOWS": "0,3"})
        self.assertEqual(proc.returncode, 0, proc.stderr)
        row = read_rows(self.out)[0]
        self.assertEqual(row["status"], "invalid")
        self.assertIsNone(row["warm"])

    def test_abort_is_typed_with_null_counts(self):
        proc = self.alloc("abort-arm")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        row = read_rows(self.out)[0]
        self.assertEqual(row["status"], "process_abort")
        self.assertIn("panicked", row["process"]["stderr_tail"])
        self.assertIsNone(row["warm"])
        self.assertIsNone(row["windows"])


if __name__ == "__main__":
    unittest.main()
