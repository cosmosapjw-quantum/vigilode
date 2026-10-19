#!/usr/bin/env python3
"""Tests of the Diffsol arms of tools/triage/triage_external.py and the `--diffsol-binary` check of
tools/triage/triage_parity.py, with a fake `diffsol-triage` executable (no Rust build, no solver).

The fake implements the three subcommands of tools/benchmarks/diffsol/triage (`list`, `run`, `parity`) for
Robertson only. `FAKE_DIFFSOL_MODE` makes `run` abort, print garbage, sleep, exit 2, print a row for another
cell, add a key, report another atol, shift the last output time or report a solver failure;
`FAKE_DIFFSOL_ATOL_SCALE` changes the listed definition; `FAKE_DIFFSOL_PERTURB` / `FAKE_DIFFSOL_Y0_SHIFT`
perturb the parity document. The fake `rodas5p` comes from tools/test_triage_runner.py.

`VIGILODE_DIFFSOL_BINARY=PATH` additionally runs the real harness binary (skipped when unset).

Run: python3 -m unittest tools.test_triage_diffsol
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
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
from test_triage_runner import FakeEnv, read_rows, write_executable  # noqa: E402

HAVE_NUMPY = importlib.util.find_spec("numpy") is not None
HAVE_SCIPY = HAVE_NUMPY and importlib.util.find_spec("scipy") is not None
REAL_BINARY = os.environ.get("VIGILODE_DIFFSOL_BINARY")

FAKE_DIFFSOL = r'''
import hashlib, json, math, os, struct, sys, time
args = sys.argv[1:]
if os.environ.get("FAKE_LOG"):
    with open(os.environ["FAKE_LOG"], "a") as h:
        h.write("diffsol " + " ".join(args) + "\n")
cmd = args[0] if args else ""
opts = dict(zip(args[1::2], args[2::2]))
hx = lambda v: struct.pack(">d", float(v)).hex()
SHA = hashlib.sha256(open(sys.argv[0], "rb").read()).hexdigest()
y0 = [1.0, 0.0, 0.0]
if os.environ.get("FAKE_DIFFSOL_Y0_SHIFT"):
    y0[0] = 1.0 + 2.0 ** -52
PROBLEMS = {"robertson": {"id": "robertson", "tier": "G", "dimension": 3, "t_span": [0.0, 40.0],
                          "atol_scale": float(os.environ.get("FAKE_DIFFSOL_ATOL_SCALE", "1e-4")),
                          "autonomous": True, "breakpoints": [], "jacobian": "analytic",
                          "y0_hex": [hx(v) for v in y0], "source": "fake"}}
mode = os.environ.get("FAKE_DIFFSOL_MODE", "")
if cmd == "list":
    print(json.dumps({"schema": "vigilode-diffsol-triage-list-v1", "diffsol": "0.16.1",
                      "methods": [{"id": m, "arm": "ext.diffsol-" + m} for m in ("bdf", "esdirk34", "tr-bdf2")],
                      "problems": list(PROBLEMS.values()), "binary_sha256": SHA}))
    sys.exit(0)
if cmd == "run":
    if opts.get("--problem") not in PROBLEMS or opts.get("--method") not in ("bdf", "esdirk34", "tr-bdf2"):
        sys.stderr.write("error: unknown problem or method\n"); sys.exit(2)
    if mode == "abort":
        sys.stderr.write("thread 'main' panicked at diffsol/src/ode_solver/method.rs: assertion\n")
        sys.stderr.flush(); os.abort()
    if mode == "garbage":
        print("not json"); sys.exit(0)
    if mode == "sleep":
        time.sleep(30)
    if mode == "usage":
        sys.stderr.write("error: bad option\n"); sys.exit(2)
    p = PROBLEMS[opts["--problem"]]
    rtol, grid = float(opts["--rtol"]), opts["--grid"]
    t0, tf = p["t_span"]
    times = [t0, tf] if grid == "endpoint" else [t0 + (tf - t0) * float(i) / 100.0 for i in range(101)]
    times[0], times[-1] = t0, tf
    if mode == "shift-time":
        times[-1] = struct.unpack(">d", struct.pack(">q", struct.unpack(">q", struct.pack(">d", tf))[0] - 1))[0]
    atol = rtol * p["atol_scale"] * (2.0 if mode == "atol" else 1.0)
    stats = {"number_of_linear_solver_setups": 9, "number_of_steps": 40, "number_of_error_test_failures": 3,
             "number_of_nonlinear_solver_iterations": 70, "number_of_nonlinear_solver_fails": 1,
             "number_of_linear_solver_setups_from_checkpoint": 1,
             "number_of_linear_solver_setups_from_first_convergence_fail": 1,
             "number_of_linear_solver_setups_from_second_convergence_fail": 0,
             "number_of_linear_solver_setups_from_error_test_fail": 3,
             "number_of_linear_solver_setups_from_step_success": 4}
    op = {"number_of_calls": 120, "number_of_jac_muls": 12, "number_of_matrix_evals": 4, "number_of_jac_adj_muls": 0}
    failed = mode == "fail"
    row = {"schema": "vigilode-triage-row-v1", "campaign": opts["--campaign"],
           "arm": "ext.diffsol-" + opts["--method"], "components": [], "problem": p["id"], "tier": p["tier"],
           "dimension": p["dimension"], "rtol": rtol * (10.0 if mode == "mismatch" else 1.0), "atol": atol,
           "grid": grid, "h0_scale": 1.0, "h0": None, "status": "solver_failure" if failed else "ok",
           "error": "solve_dense: ODE solver error: Step size is too small at time = 1" if failed else None,
           "success": not failed, "attempts": 44, "accepted_steps": 40, "rejected_steps": 4,
           "output_times": [] if failed else times,
           "states_hex": [] if failed else [[hx(1.0), hx(0.0), hx(0.0)] for _ in times], "counters": None,
           "extra_work": {"upstream": "diffsol 0.16.1", "method": opts["--method"], "solver_statistics": stats,
                          "rhs_op_statistics": op, "nfev": 120, "njev": 4, "nlu": 9, "h0_policy": "solver_default"},
           "counter_semantics": "external-upstream-raw", "deterministic": True,
           "repetitions": int(opts.get("--repetitions", "1")), "binary": {"sha256": SHA, "profile": "external"},
           "source": {"commit": os.environ.get("VIGILODE_SOURCE_COMMIT"),
                      "tree_clean": {"true": True, "false": False}.get(os.environ.get("VIGILODE_TREE_CLEAN"))}}
    if mode == "extra-key":
        row["wall_seconds"] = 0.1
    print(json.dumps(row)); sys.exit(0)
if cmd == "parity":
    raw = open(opts["--input"], "rb").read()
    doc = json.loads(raw)
    if doc["problem"] not in PROBLEMS:
        sys.stderr.write("error: unknown problem\n"); sys.exit(2)
    eps = float(os.environ.get("FAKE_DIFFSOL_PERTURB", "0"))
    un = lambda h: struct.unpack(">d", bytes.fromhex(h))[0]
    v = [un(h) for h in doc["jvp_direction_hex"]]
    samples = []
    for s in doc["samples"]:
        y = [un(h) for h in s["y_hex"]]
        f = [-0.04 * y[0] + 1.0e4 * y[1] * y[2], 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1],
             3.0e7 * y[1] * y[1]]
        f[0] *= 1.0 + eps
        jac = [[-0.04, 1.0e4 * y[2], 1.0e4 * y[1]], [0.04, -1.0e4 * y[2] - 6.0e7 * y[1], -1.0e4 * y[1]],
               [0.0, 6.0e7 * y[1], 0.0]]
        jvp = [jac[r][0] * v[0] + jac[r][1] * v[1] + jac[r][2] * v[2] for r in range(3)]
        samples.append({"t_hex": s["t_hex"], "y_hex": s["y_hex"], "f_hex": [hx(x) for x in f], "f_error": None,
                        "jacobian": {"format": "dense", "rows_hex": [[hx(x) for x in r] for r in jac]},
                        "jvp_hex": [hx(x) for x in jvp]})
    p = PROBLEMS[doc["problem"]]
    print(json.dumps({"schema": "vigilode-triage-parity-v1", "problem": doc["problem"], "dimension": 3,
                      "autonomous": True, "jacobian": "analytic", "jvp_direction_hex": doc["jvp_direction_hex"],
                      "mass_hex": None, "samples": samples,
                      "diffsol": {"input_sha256": hashlib.sha256(raw).hexdigest(), "y0_hex": p["y0_hex"],
                                  "t_span": p["t_span"], "atol_scale": p["atol_scale"], "source": "fake"}}))
    sys.exit(0)
sys.stderr.write("usage\n"); sys.exit(2)
'''

DIFFSOL_ARMS = ("ext.diffsol-bdf", "ext.diffsol-esdirk34", "ext.diffsol-tr-bdf2")


class DiffsolTmp(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="triage_diffsol_")
        self.fake = FakeEnv(Path(self._tmp.name))
        self.root = self.fake.root
        self.diffsol = write_executable(self.root / "diffsol-triage", FAKE_DIFFSOL)

    def tearDown(self):
        self._tmp.cleanup()

    def external(self, *extra: str, out: Path | None = None, env: dict | None = None, arms=DIFFSOL_ARMS,
                 problems=("robertson",)):
        args = ["--corpus", str(self.fake.corpus), "--campaign", "ct-fake", "--output",
                str(out or self.root / "EXTERNAL.jsonl")]
        for arm in arms:
            args += ["--external-arm", arm]
        for problem in problems:
            args += ["--problem", problem]
        return self.fake.run("triage_external.py", *args, *extra, extra_env=env)


class ExternalDiffsolTests(DiffsolTmp):
    def test_registry_has_the_three_arms(self):
        import triage_external as te
        for arm in DIFFSOL_ARMS:
            self.assertIn(arm, te.EXTERNAL_ARMS)
        self.assertEqual(te.DIFFSOL_ARMS, {"ext.diffsol-bdf": "bdf", "ext.diffsol-esdirk34": "esdirk34",
                                           "ext.diffsol-tr-bdf2": "tr-bdf2"})

    def test_no_binary_gives_typed_unavailable_rows(self):
        for extra, needle in (((), "Diffsol harness binary not given"),
                              (("--diffsol-binary", str(self.root / "missing")), "not found")):
            with self.subTest(extra=extra):
                out = self.root / f"EXT_{len(extra)}.jsonl"
                proc = self.external(*extra, out=out)
                self.assertEqual(proc.returncode, 0, proc.stderr)
                rows = read_rows(out)
                self.assertEqual(len(rows), 3 * 2)  # 3 arms x 2 rtols, endpoint grid, nominal h0 only
                self.assertEqual({r["status"] for r in rows}, {"unavailable"})
                self.assertTrue(all(needle in r["error"] for r in rows), [r["error"] for r in rows])
                import triage_check as tch
                for r in rows:
                    self.assertEqual(tch.validate_row(r, "external"), [])

    def test_rows_from_the_harness(self):
        proc = self.external("--diffsol-binary", str(self.diffsol), problems=("robertson", "wide"))
        self.assertEqual(proc.returncode, 0, proc.stderr)
        rows = read_rows(self.root / "EXTERNAL.jsonl")
        self.assertEqual(len(rows), 3 * 2 * 2)
        import triage_check as tch
        sha = hashlib.sha256(self.diffsol.read_bytes()).hexdigest()
        for r in rows:
            self.assertEqual(tch.validate_row(r, "external"), [])
            self.assertNotIn("process", r)
            if r["problem"] == "wide":
                self.assertEqual(r["status"], "unavailable")
                self.assertIn("problem not implemented in the Diffsol harness", r["error"])
                continue
            self.assertEqual(r["status"], "ok", r["error"])
            self.assertIsNone(r["h0"])
            self.assertEqual(r["components"], [])
            self.assertIsNone(r["counters"])
            self.assertEqual(r["counter_semantics"], "external-upstream-raw")
            self.assertEqual(r["binary"], {"sha256": sha, "profile": "external"})
            self.assertEqual(r["output_times"], [0.0, 40.0])
            self.assertEqual((r["extra_work"]["nfev"], r["extra_work"]["njev"], r["extra_work"]["nlu"]), (120, 4, 9))
            self.assertEqual((r["accepted_steps"], r["rejected_steps"], r["attempts"]), (40, 4, 44))
            self.assertEqual(r["atol"], r["rtol"] * 1e-4)
        commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
        ok = [r for r in rows if r["status"] == "ok"]
        self.assertEqual({r["source"]["commit"] for r in ok}, {commit})
        self.assertEqual(sorted({r["arm"] for r in ok}), sorted(DIFFSOL_ARMS))
        calls = self.fake.calls("diffsol")
        self.assertEqual(calls.count("diffsol list"), 1)
        self.assertIn("diffsol run --problem robertson --method tr-bdf2 --rtol 1e-06 --grid endpoint "
                      "--campaign ct-fake --repetitions 1", calls)
        self.assertFalse(any("wide" in c for c in calls))  # unsupported problems are never spawned
        # Resumable: a second invocation runs nothing.
        proc = self.external("--diffsol-binary", str(self.diffsol), problems=("robertson", "wide"))
        self.assertIn("12 planned external cells, 0 to run", proc.stderr)

    def test_failures_are_typed_rows(self):
        cases = {  # env -> (status, process kind or None, error needle)
            "abort": ({"FAKE_DIFFSOL_MODE": "abort"}, "process_abort", "abort", "SIGABRT"),
            "garbage": ({"FAKE_DIFFSOL_MODE": "garbage"}, "process_abort", "unparseable_row", "without a"),
            "extra-key": ({"FAKE_DIFFSOL_MODE": "extra-key"}, "process_abort", "unparseable_row", "key set"),
            "usage": ({"FAKE_DIFFSOL_MODE": "usage"}, "process_abort", "usage_exit_2", "exit status 2"),
            "mismatch": ({"FAKE_DIFFSOL_MODE": "mismatch"}, "process_abort", "cell_mismatch", "rtol"),
            "sleep": ({"FAKE_DIFFSOL_MODE": "sleep"}, "process_abort", "timeout", "timeout after 1.0 s"),
            "atol": ({"FAKE_DIFFSOL_MODE": "atol"}, "unavailable", None, "diffsol atol"),
            "shift-time": ({"FAKE_DIFFSOL_MODE": "shift-time"}, "error", None, "differ from the requested grid"),
            "fail": ({"FAKE_DIFFSOL_MODE": "fail"}, "solver_failure", None, "Step size is too small"),
            "definition": ({"FAKE_DIFFSOL_ATOL_SCALE": "1e-3"}, "unavailable", None, "definition differs"),
        }
        import triage_check as tch
        for name, (env, status, kind, needle) in cases.items():
            with self.subTest(case=name):
                out = self.root / f"EXT_{name}.jsonl"
                proc = self.external("--diffsol-binary", str(self.diffsol), "--diffsol-timeout", "1", "--rtol",
                                     "1e-06", out=out, env=env, arms=("ext.diffsol-bdf",))
                self.assertEqual(proc.returncode, 0, proc.stderr)
                rows = read_rows(out)
                self.assertEqual(len(rows), 1)
                row = rows[0]
                self.assertEqual(row["status"], status, row["error"])
                self.assertFalse(row["success"])
                self.assertIn(needle, row["error"] + json.dumps(row.get("process")))
                self.assertEqual(tch.validate_row(row, "external"), [])
                if kind is None:
                    self.assertNotIn("process", row)
                else:
                    self.assertEqual(row["process"]["kind"], kind)
                if name == "fail":
                    self.assertEqual(row["extra_work"]["nfev"], 120)
                    self.assertEqual(row["states_hex"], [])


@unittest.skipUnless(HAVE_NUMPY, "numpy not installed")
class ParityDiffsolTests(DiffsolTmp):
    def parity(self, *extra: str, env: dict | None = None, name: str = "PARITY.json"):
        out = self.root / name
        proc = self.fake.run("triage_parity.py", "--binary", str(self.fake.binary), "--corpus", str(self.fake.corpus),
                             "--output", str(out), *extra, extra_env=env)
        doc = json.loads(out.read_text()) if out.exists() else None
        return proc, doc

    def test_diffsol_only_pass_fail_and_not_checked(self):
        cases = {  # env -> (exit, robertson status, reason needle)
            "pass": ({}, 0, "pass", None),
            "perturbed": ({"FAKE_DIFFSOL_PERTURB": "1e-12"}, 1, "fail", "rhs max rel"),
            "tiny": ({"FAKE_DIFFSOL_PERTURB": "1e-15"}, 0, "pass", None),
            "y0": ({"FAKE_DIFFSOL_Y0_SHIFT": "1"}, 1, "fail", "differs from the Diffsol initial state"),
            "definition": ({"FAKE_DIFFSOL_ATOL_SCALE": "1e-3"}, 1, "fail", "atol_scale"),
        }
        for name, (env, code, status, needle) in cases.items():
            with self.subTest(case=name):
                proc, doc = self.parity("--diffsol-binary", str(self.diffsol), "--diffsol-only", env=env,
                                        name=f"PARITY_{name}.json")
                self.assertEqual(proc.returncode, code, proc.stderr)
                self.assertEqual(doc["problems"], [])
                self.assertEqual(doc["numpy_models"], "skipped (--diffsol-only)")
                self.assertEqual(doc["limit"], 1e-13)  # frozen THRESHOLDS parity.max_relative_error
                by = {p["problem"]: p for p in doc["diffsol"]["problems"]}
                self.assertEqual(doc["diffsol"]["binary_sha256"], hashlib.sha256(self.diffsol.read_bytes()).hexdigest())
                r = by["robertson"]
                self.assertEqual(r["status"], status, r["reason"])
                self.assertEqual(r["compared"], ["rhs", "jacobian", "jvp"])
                self.assertEqual(r["samples"], 4)
                if needle is not None:
                    self.assertIn(needle, r["reason"])
                if name == "perturbed":
                    self.assertEqual((r["worst"]["kind"], r["worst"]["index"]), ("rhs", 0))
                    self.assertIn("diffsol_hex", r["worst"])
                self.assertEqual((by["wide"]["status"], by["wide"]["reason"]),
                                 ("not_checked", "problem not implemented in the Diffsol harness"))

    def test_diffsol_only_needs_the_binary(self):
        proc, doc = self.parity("--diffsol-only")
        self.assertEqual(proc.returncode, 2)
        self.assertIn("--diffsol-only needs --diffsol-binary", proc.stderr)
        self.assertIsNone(doc)

    @unittest.skipUnless(HAVE_SCIPY, "the NumPy-model check needs scipy")
    def test_both_checks_in_one_report(self):
        proc, doc = self.parity("--diffsol-binary", str(self.diffsol), "--problem", "robertson")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(doc["problems"][0]["status"], "pass")
        self.assertEqual(doc["diffsol"]["problems"][0]["status"], "pass")
        self.assertNotIn("numpy_models", doc)


@unittest.skipUnless(REAL_BINARY, "set VIGILODE_DIFFSOL_BINARY to the built tools/benchmarks/diffsol/triage binary")
class RealHarnessTests(unittest.TestCase):
    def test_list_run_and_row_contract(self):
        import triage_check as tch
        import triage_common as tc
        listing = json.loads(subprocess.run([REAL_BINARY, "list"], capture_output=True, text=True, check=True).stdout)
        self.assertEqual(listing["schema"], "vigilode-diffsol-triage-list-v1")
        self.assertEqual(len(listing["problems"]), 9)
        proc = subprocess.run([REAL_BINARY, "run", "--problem", "robertson", "--method", "bdf", "--rtol", "1e-06",
                               "--grid", "uniform101", "--campaign", "ct-test", "--repetitions", "2"],
                              capture_output=True, text=True, check=True)
        row = json.loads(proc.stdout)
        self.assertEqual(set(row), set(tc.ROW_KEYS))
        self.assertEqual(tch.validate_row(row, "real"), [])
        self.assertEqual(row["status"], "ok", row["error"])
        self.assertTrue(row["deterministic"])
        self.assertEqual(row["output_times"], tc.grid_times([0.0, 40.0], "uniform101"))
        self.assertEqual(row["atol"], 1e-6 * 1e-4)
        work = row["extra_work"]
        self.assertTrue(all(work["mapping_check"].values()), work["mapping_check"])
        self.assertEqual(work["nfev"], work["rhs_op_statistics"]["number_of_calls"])
        self.assertEqual(work["nlu"], work["solver_statistics"]["number_of_linear_solver_setups"])
        self.assertEqual(row["accepted_steps"], work["solver_statistics"]["number_of_steps"])
        y = struct.unpack(">d", bytes.fromhex(row["states_hex"][-1][0]))[0]
        self.assertAlmostEqual(y, 0.7158270687, delta=1e-5)
        bad = subprocess.run([REAL_BINARY, "run", "--problem", "nope", "--method", "bdf", "--rtol", "1e-6", "--grid",
                              "endpoint", "--campaign", "x"], capture_output=True, text=True)
        self.assertEqual(bad.returncode, 2)


if __name__ == "__main__":
    unittest.main()
