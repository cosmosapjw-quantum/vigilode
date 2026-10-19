#!/usr/bin/env python3
"""Tests of tools/triage/triage_runner.py with a fake `rodas5p` executable (no Rust build, no solver).

The fake binary (written to a temporary directory) implements the CT-HARNESS v1 subcommands `triage-list`,
`triage-run`, `triage-profile-run`, `triage` (allocation audit), `triage-parity` and `triage-exact` well enough
to exercise the drivers: an arm `abort-arm` aborts (SIGABRT) after writing a panic message, `garbage-arm`
prints no JSON, `mismatch-arm` prints a row for another rtol, unknown arms exit 2. Every invocation is logged.
The fakes are shared with tools/test_triage_profile.py and tools/test_triage_tools.py.

Run: python3 -m pytest tools/test_triage_runner.py -q
"""

from __future__ import annotations

import json
import os
import stat
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

FAKE_RODAS5P = r'''
import json, os, struct, sys
args = sys.argv[1:]
if os.environ.get("FAKE_LOG"):
    with open(os.environ["FAKE_LOG"], "a") as h:
        h.write(" ".join(args) + "\n")
cmd = args[0] if args else ""
opts = dict(zip(args[1::2], args[2::2]))
LIST = json.load(open(os.environ["FAKE_LIST"]))
ARMS = {a["id"]: a for a in LIST["arms"]}
PROBLEMS = {p["id"]: p for p in LIST["problems"]}
hx = lambda v: struct.pack(">d", float(v)).hex()
if cmd == "triage-list":
    print(json.dumps(LIST)); sys.exit(0)
if cmd in ("triage-run", "triage-profile-run", "triage"):
    arm, problem = opts.get("--arm"), opts.get("--problem")
    if arm not in ARMS or problem not in PROBLEMS:
        sys.stderr.write("unknown arm or problem\n"); sys.exit(2)
    if arm == "abort-arm":
        sys.stderr.write("thread 'main' panicked at src/x.rs: boom\n"); sys.stderr.flush(); os.abort()
    if arm == "garbage-arm":
        print("this is not json"); sys.exit(0)
    rtol, h0s, grid = float(opts["--rtol"]), float(opts["--h0-scale"]), opts["--grid"]
    p = PROBLEMS[problem]
    if cmd == "triage-profile-run":
        print(json.dumps({"success": True, "attempts": 12, "counters": {"rhs_calls": 3}, "deterministic": True}))
        sys.exit(0)
    if cmd == "triage":
        windows = [int(w) for w in os.environ.get("FAKE_WINDOWS", "0").split(",")]
        w = lambda n: {"allocations": n, "deallocations": n, "allocated_bytes": 16 * n, "peak_live_bytes": -n}
        print(json.dumps({"schema": "vigilode-triage-alloc-v1", "arm": arm, "problem": problem, "rtol": rtol,
                          "grid": grid, "h0_scale": h0s, "warmups": int(opts["--warmups"]), "cold": w(40),
                          "windows": [w(n) for n in windows][: int(opts["--windows"])]}))
        sys.exit(0)
    t0, tf = p["t_span"]
    times = [t0, tf] if grid == "endpoint" else [t0 + (tf - t0) * i / 100.0 for i in range(101)]
    row = {"schema": "vigilode-triage-row-v1", "campaign": opts.get("--campaign"), "arm": arm,
           "components": ARMS[arm]["components"], "problem": problem, "tier": p["tier"], "dimension": p["dimension"],
           "rtol": rtol * (10.0 if arm == "mismatch-arm" else 1.0), "atol": rtol * p["atol_scale"], "grid": grid,
           "h0_scale": h0s, "h0": h0s * 1e-6 * (tf - t0), "status": "ok", "error": None, "success": True,
           "attempts": 12, "accepted_steps": 10, "rejected_steps": 2, "output_times": times,
           "states_hex": [[hx(1.0)] * p["dimension"] for _ in times], "counters": {"rhs_calls": 3},
           "extra_work": None, "counter_semantics": "work-counters-v2-attempted", "deterministic": True,
           "repetitions": int(opts.get("--repetitions", "1")),
           "binary": {"sha256": "ab" * 32, "profile": os.environ.get("FAKE_PROFILE", "bench")},
           "source": {"commit": os.environ.get("VIGILODE_SOURCE_COMMIT"),
                      "tree_clean": {"true": True, "false": False}.get(os.environ.get("VIGILODE_TREE_CLEAN"))}}
    print(json.dumps(row)); sys.exit(0)
if cmd == "triage-parity":
    # The layout of crates/rodas5p-cli/src/triage.rs::parity_json (robertson only). FAKE_PARITY_MODE:
    # dense (default), coo, jvp (no Jacobian, J v only), f_error (Rust f fails on sample 1).
    problem = opts["--problem"]
    if problem != "robertson":
        sys.stderr.write("no parity samples\n"); sys.exit(2)
    import math
    mode = os.environ.get("FAKE_PARITY_MODE", "dense")
    eps = float(os.environ.get("FAKE_PARITY_PERTURB", "0"))
    y0 = [1.0, 0.0, 0.0]
    if os.environ.get("FAKE_PARITY_Y0_SHIFT"):
        y0[0] = 1.0 + 2.0 ** -52
    v = [math.sin(i + 1.0) for i in range(3)]
    states = [y0, [y * (1.0 + 0.1 * math.sin(i + 1.0)) + 0.01 * math.cos(i + 1.0) for i, y in enumerate(y0)]]
    samples = []
    for t in (0.0, 20.0):
        for k, y in enumerate(states):
            f = [-0.04 * y[0] + 1.0e4 * y[1] * y[2], 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1],
                 3.0e7 * y[1] * y[1]]
            f[0] *= 1.0 + eps
            jac = [[-0.04, 1.0e4 * y[2], 1.0e4 * y[1]], [0.04, -1.0e4 * y[2] - 6.0e7 * y[1], -1.0e4 * y[1]],
                   [0.0, 6.0e7 * y[1], 0.0]]
            jvp = [jac[r][0] * v[0] + jac[r][1] * v[1] + jac[r][2] * v[2] for r in range(3)]
            sample = {"t_hex": hx(t), "y_hex": [hx(x) for x in y], "f_hex": [hx(x) for x in f], "f_error": None,
                      "jacobian": {"format": "dense", "rows_hex": [[hx(x) for x in r] for r in jac]},
                      "jvp_hex": [hx(x) for x in jvp]}
            if mode == "coo":
                sample["jacobian"] = {"format": "coo", "nonzeros_hex": [[r, c, hx(jac[r][c])] for r in range(3)
                                                                         for c in range(3) if jac[r][c] != 0.0]}
            elif mode == "jvp":
                sample["jacobian"] = None
            elif mode == "f_error" and k == 1:
                sample.update(f_hex=None, f_error="callback error: state outside the domain")
            samples.append(sample)
    mass = [[hx(1.0), hx(0.0), hx(0.0)]] * 3 if os.environ.get("FAKE_PARITY_MASS") else None
    print(json.dumps({"schema": "vigilode-triage-parity-v1", "problem": problem, "dimension": 3,
                      "autonomous": True, "jacobian": "jvp" if mode == "jvp" else "analytic",
                      "jvp_direction_hex": [hx(x) for x in v], "mass_hex": mass, "samples": samples}))
    sys.exit(0)
if cmd == "triage-exact":
    # The layout of triage.rs (closed form y = exp(-t) in every component; t_span from the problem id table).
    import math
    problem, grid = opts["--problem"], opts["--grid"]
    t0, tf = 0.0, 40.0
    times = [t0, tf] if grid == "endpoint" else [t0 + (tf - t0) * float(i) / 100.0 for i in range(101)]
    times[0], times[-1] = t0, tf
    if os.environ.get("FAKE_EXACT_SHIFT"):
        times[-1] = struct.unpack(">d", struct.pack(">q", struct.unpack(">q", struct.pack(">d", tf))[0] - 1))[0]
    print(json.dumps({"schema": "vigilode-triage-exact-v1", "problem": problem, "grid": grid, "dimension": 3,
                      "output_times": times, "output_times_hex": [hx(t) for t in times],
                      "states_hex": [[hx(math.exp(-t))] * 3 for t in times]}))
    sys.exit(0)
sys.stderr.write("usage\n"); sys.exit(2)
'''

FAKE_VALGRIND = r'''
import os, subprocess, sys
args = sys.argv[1:]
if args == ["--version"]:
    print("valgrind-3.99.fake"); sys.exit(0)
tool = out = None
i = 0
while args[i].startswith("--"):
    if args[i].startswith("--tool="):
        tool = args[i].split("=", 1)[1]
    elif args[i].startswith(("--callgrind-out-file=", "--cachegrind-out-file=")):
        out = args[i].split("=", 1)[1]
    i += 1
command = args[i:]
k = int(command[command.index("--repetitions") + 1])
if os.environ.get("FAKE_VALGRIND_FAIL"):
    sys.stderr.write("valgrind: fatal error\n"); sys.exit(1)
proc = subprocess.run(command, capture_output=True, text=True)
sys.stdout.write(proc.stdout)
extra = 0
if tool == "callgrind" and k == 1 and os.environ.get("FAKE_STATE"):
    state = os.environ["FAKE_STATE"]
    seen = int(open(state).read()) if os.path.exists(state) else 0
    open(state, "w").write(str(seen + 1))
    if seen >= 1 and os.environ.get("FAKE_NONDET"):
        extra = 1
ir = (4000 if k == 0 else 5000 + 1300 * k) + extra
with open(out, "w") as h:
    if tool == "callgrind":
        h.write(f"version: 1\ncmd: fake\nevents: Ir\nsummary: {ir}\n\nfn=main\n1 {ir}\ntotals: {ir}\n")
    else:
        h.write(f"desc: fake\ncmd: fake\nevents: Ir I1mr D1mr DLmr\nfl=x\nfn=main\n1 {ir} {k} {10 * k + 3} {2 * k}\n"
                f"summary: {ir} {k} {10 * k + 3} {2 * k}\n")
sys.exit(proc.returncode)
'''

LIST = {
    "schema": "vigilode-triage-list-v1",
    "arms": [
        {"id": "ok-arm", "components": ["drv.ok", "la.x"], "wired": True, "unwired_reason": None,
         "requires": {"jacobian": "analytic", "structure": None, "mass": False, "fixed_dimension": None}},
        {"id": "abort-arm", "components": ["drv.abort", "la.x"], "wired": True, "unwired_reason": None,
         "requires": {"jacobian": "analytic", "structure": None, "mass": False, "fixed_dimension": None}},
        {"id": "garbage-arm", "components": ["drv.garbage", "la.x"], "wired": True, "unwired_reason": None,
         "requires": {"jacobian": "any", "structure": None, "mass": False, "fixed_dimension": None}},
        {"id": "mismatch-arm", "components": ["drv.mismatch", "la.x"], "wired": True, "unwired_reason": None,
         "requires": {"jacobian": "any", "structure": None, "mass": False, "fixed_dimension": None}},
        {"id": "banded-arm", "components": ["drv.banded", "la.banded"], "wired": True, "unwired_reason": None,
         "requires": {"jacobian": "analytic", "structure": "banded", "mass": False, "fixed_dimension": None}},
        {"id": "unwired-arm", "components": ["drv.unwired"], "wired": False, "unwired_reason": "no entry point",
         "requires": {"jacobian": "any", "structure": None, "mass": False, "fixed_dimension": None}},
    ],
    "problems": [
        {"id": "robertson", "tier": "G", "dimension": 3, "structure": "dense", "jacobian": "analytic", "jvp": True,
         "mass": False, "exact_solution": False, "oscillatory": False, "t_span": [0.0, 40.0], "atol_scale": 1e-4,
         "breakpoints": [], "source": "fake"},
        {"id": "wide", "tier": "P", "dimension": 500, "structure": "dense", "jacobian": "analytic", "jvp": True,
         "mass": False, "exact_solution": False, "oscillatory": False, "t_span": [0.0, 1.0], "atol_scale": 1.0,
         "breakpoints": [], "source": "fake"},
    ],
}
CORPUS = {
    "schema": "vigilode-triage-corpus-v1",
    "problems": [
        {"id": "robertson", "tier": "G", "partition": "calibration", "dimension": 3, "structure": "dense",
         "jacobian": "analytic", "mass": False, "exact_solution": False, "oscillatory": False, "t_span": [0.0, 40.0],
         "atol_scale": 1e-4, "reference": {"kind": "two_method", "methods": ["Radau", "LSODA"]}},
        {"id": "wide", "tier": "P", "partition": "holdout", "dimension": 500, "structure": "dense",
         "jacobian": "analytic", "mass": False, "exact_solution": False, "oscillatory": False, "t_span": [0.0, 1.0],
         "atol_scale": 1.0, "reference": {"kind": "two_method", "methods": ["Radau", "LSODA"]}},
    ],
    "sweep": {"rtols": [1e-4, 1e-6], "grids": ["endpoint"], "h0_scales": [0.5, 1.0, 2.0]},
    "profile_subset": {"rtols": [1e-6], "max_dimension": 100},
}


def write_executable(path: Path, body: str) -> Path:
    path.write_text(f"#!{sys.executable}\n" + body)
    path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return path


class FakeEnv:
    """Fake binaries, triage-list, CORPUS and a log in one temporary directory."""

    def __init__(self, root: Path):
        self.root = root
        self.binary = write_executable(root / "rodas5p", FAKE_RODAS5P)
        self.valgrind = write_executable(root / "valgrind", FAKE_VALGRIND)
        self.list = root / "list.json"
        self.list.write_text(json.dumps(LIST))
        self.corpus = root / "CORPUS.json"
        self.corpus.write_text(json.dumps(CORPUS))
        self.log = root / "calls.log"
        self.env = dict(os.environ, FAKE_LIST=str(self.list), FAKE_LOG=str(self.log), PYTHONDONTWRITEBYTECODE="1")

    def calls(self, prefix: str = "") -> list[str]:
        if not self.log.exists():
            return []
        return [line for line in self.log.read_text().splitlines() if line.startswith(prefix)]

    def run(self, script: str, *args: str, extra_env: dict | None = None) -> subprocess.CompletedProcess:
        env = dict(self.env, **(extra_env or {}))
        return subprocess.run([sys.executable, str(TRIAGE / script), *args], capture_output=True, text=True,
                              env=env, timeout=300)


def read_rows(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="triage_runner_")
        self.fake = FakeEnv(Path(self._tmp.name))
        self.out = self.fake.root / "RUNS.jsonl"

    def tearDown(self):
        self._tmp.cleanup()

    def runner(self, *extra: str, out: Path | None = None) -> subprocess.CompletedProcess:
        return self.fake.run("triage_runner.py", "--binary", str(self.fake.binary), "--list", str(self.fake.list),
                             "--corpus", str(self.fake.corpus), "--campaign", "ct-fake", "--output",
                             str(out or self.out), *extra)

    def test_help(self):
        proc = self.fake.run("triage_runner.py", "--help")
        self.assertEqual(proc.returncode, 0)
        self.assertIn("process_abort", proc.stdout)

    def test_every_cell_gets_a_row_and_aborts_are_typed(self):
        proc = self.runner("--problem", "robertson")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        rows = read_rows(self.out)
        # 4 wired applicable arms (banded-arm is inapplicable, unwired-arm unwired) x 2 rtols x 3 h0 on robertson
        self.assertEqual(len(rows), 4 * 2 * 3)
        by_arm = {}
        for r in rows:
            by_arm.setdefault(r["arm"], []).append(r)
        self.assertEqual(sorted(by_arm), ["abort-arm", "garbage-arm", "mismatch-arm", "ok-arm"])
        self.assertTrue(all(r["status"] == "ok" for r in by_arm["ok-arm"]))
        abort = by_arm["abort-arm"][0]
        self.assertEqual(abort["status"], "process_abort")
        self.assertEqual(abort["process"]["kind"], "abort")
        self.assertLess(abort["process"]["exit_status"], 0)
        self.assertIn("SIGABRT", abort["process"]["signal"])
        self.assertIn("panicked", abort["process"]["stderr_tail"])
        self.assertEqual(abort["components"], ["drv.abort", "la.x"])
        self.assertFalse(abort["success"])
        self.assertEqual(abort["states_hex"], [])
        self.assertEqual(by_arm["garbage-arm"][0]["process"]["kind"], "unparseable_row")
        self.assertEqual(by_arm["mismatch-arm"][0]["process"]["kind"], "cell_mismatch")
        self.assertIn("rtol", by_arm["mismatch-arm"][0]["error"])
        for r in rows:  # contract key set on every row, aborts included
            self.assertTrue({"schema", "campaign", "arm", "status", "states_hex", "source"} <= set(r))
        commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
        self.assertEqual(by_arm["ok-arm"][0]["source"]["commit"], commit)
        self.assertIn(by_arm["ok-arm"][0]["source"]["tree_clean"], (True, False))

    def test_usage_exit_2_is_typed(self):
        # A CORPUS/list problem the binary does not know: triage-run exits 2 for every cell of it.
        listing = json.loads(self.fake.list.read_text())
        corpus = json.loads(self.fake.corpus.read_text())
        listing["problems"].append(dict(listing["problems"][0], id="ghost"))
        corpus["problems"].append(dict(corpus["problems"][0], id="ghost"))
        self.fake.corpus.write_text(json.dumps(corpus))
        runner_list = self.fake.root / "list_with_ghost.json"
        runner_list.write_text(json.dumps(listing))
        proc = self.fake.run("triage_runner.py", "--binary", str(self.fake.binary), "--list", str(runner_list),
                             "--corpus", str(self.fake.corpus), "--campaign", "ct-fake", "--output", str(self.out),
                             "--problem", "ghost", "--arm", "ok-arm")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        rows = read_rows(self.out)
        self.assertEqual(len(rows), 6)
        self.assertEqual({r["process"]["kind"] for r in rows}, {"usage_exit_2"})
        self.assertEqual({r["process"]["exit_status"] for r in rows}, {2})

    def test_resumable_and_deterministic_with_jobs(self):
        self.assertEqual(self.runner("--arm", "ok-arm", "--arm", "abort-arm").returncode, 0)
        first = self.out.read_bytes()
        calls = len(self.fake.calls("triage-run"))
        self.assertEqual(calls, 2 * 2 * 2 * 3)
        # Second run: everything present (aborted cells included), nothing runs, file unchanged.
        self.assertEqual(self.runner("--arm", "ok-arm", "--arm", "abort-arm").returncode, 0)
        self.assertEqual(len(self.fake.calls("triage-run")), calls)
        self.assertEqual(self.out.read_bytes(), first)
        # Drop the last two rows (an interrupted campaign); only those cells run again, in plan order.
        lines = first.decode().splitlines(True)
        self.out.write_text("".join(lines[:-2]))
        self.assertEqual(self.runner("--arm", "ok-arm", "--arm", "abort-arm").returncode, 0)
        self.assertEqual(len(self.fake.calls("triage-run")), calls + 2)
        self.assertEqual(self.out.read_bytes(), first)
        # -j 4 writes the same bytes in the same order.
        parallel = self.fake.root / "PARALLEL.jsonl"
        self.assertEqual(self.runner("--arm", "ok-arm", "--arm", "abort-arm", "-j", "4", out=parallel).returncode, 0)
        self.assertEqual(parallel.read_bytes(), first)

    def test_state_sidecars_with_gz_references(self):
        import gzip
        import struct as st
        import triage_common as tc
        hx1 = st.pack(">d", 1.0).hex()
        refs = {"schema": "vigilode-triage-references-v1", "references": [
            {"problem": "wide", "grid": "endpoint", "kind": "two_method", "status": "ok", "reason": None,
             "output_times": [0.0, 1.0], "states_hex": [[hx1] * 500] * 2,
             "rho_hex": [[st.pack(">d", 0.0).hex()] * 500] * 2, "rho_rule": "x", "methods": [],
             "canonical_method": "Radau", "model_source": "test"}]}
        gz = self.fake.root / "REFERENCES.json.gz"
        gz.write_bytes(gzip.compress(json.dumps(refs).encode(), mtime=0))
        states = self.fake.root / "states"
        proc = self.runner("--arm", "ok-arm", "--rtol", "1e-06", "--h0-scale", "1.0", "--references", str(gz),
                           "--states-dir", str(states))
        self.assertEqual(proc.returncode, 0, proc.stderr)
        rows = {r["problem"]: r for r in read_rows(self.out)}
        wide, small = rows["wide"], rows["robertson"]
        self.assertIn("states_hex", small)  # dimension 3 <= 64: unchanged
        self.assertNotIn("accuracy_inline", small)
        self.assertNotIn("states_hex", wide)
        self.assertEqual(wide["states_shape"], [2, 500])
        loaded, source, _ = tc.load_states_sidecar([states], wide)
        self.assertEqual(source, "sidecar")
        self.assertEqual(loaded, [[1.0] * 500] * 2)
        inline = wide["accuracy_inline"]
        self.assertEqual((inline["class"], inline["E"], inline["R_up"]), ("computed", 0.0, 0.0))
        self.assertEqual(inline["reference_sha256"], tc.reference_entry_sha256(refs["references"][0]))
        self.assertEqual(inline["function"], tc.ACCURACY_FUNCTION)
        # Without --states-dir: states stay inline, with a warning.
        plain = self.fake.root / "PLAIN.jsonl"
        proc = self.runner("--arm", "ok-arm", "--rtol", "1e-06", "--h0-scale", "1.0", "--problem", "wide", out=plain)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("keep states_hex inline", proc.stderr)
        self.assertIn("states_hex", read_rows(plain)[0])
        # A reference-less large row still moves, with a typed unknown inline accuracy.
        bare = self.fake.root / "BARE.jsonl"
        proc = self.runner("--arm", "ok-arm", "--rtol", "1e-06", "--h0-scale", "1.0", "--problem", "wide",
                           "--states-dir", str(states), out=bare)
        inline = read_rows(bare)[0]["accuracy_inline"]
        self.assertEqual((inline["class"], inline["reason"], inline["reference_sha256"]),
                         ("unknown", "reference: reference_missing", None))

    def test_force_truncates(self):
        self.assertEqual(self.runner("--arm", "ok-arm", "--rtol", "1e-06").returncode, 0)
        self.assertEqual(len(read_rows(self.out)), 2 * 3)
        self.assertEqual(self.runner("--arm", "ok-arm", "--rtol", "1e-04", "--force").returncode, 0)
        rows = read_rows(self.out)
        self.assertEqual({r["rtol"] for r in rows}, {1e-4})


if __name__ == "__main__":
    unittest.main()
