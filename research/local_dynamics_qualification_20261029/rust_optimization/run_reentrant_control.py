#!/usr/bin/env python3
"""Bounded exact-parity timing/profile for the reentrant sentinel control.

No builds are performed. The caller serializes this runner under the native
benchmark lock; all child attempts are recorded in a create-only output tree.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import time

CASE = "reentrant-nonlinear"
POLICIES = ("rhs-only", "observed-strict-sentinel")
REPEATS = 32
CHILD_TIMEOUT = 90
TOTAL_BUDGET = 600


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def put(path, payload):
    with Path(path).open("xb") as f:
        f.write(payload)


def callgrind_summary(path):
    events = None
    summary = None
    for line in Path(path).read_text(errors="replace").splitlines():
        if line.startswith("events:"):
            events = line.partition(":")[2].split()
        elif line.startswith("summary:"):
            try:
                summary = [int(x) for x in line.partition(":")[2].split()]
            except ValueError:
                pass
    return {"events": events, "summary": summary}


def canon(row):
    # Exclude only the measured time. json.dumps preserves the sign of -0.0.
    row = dict(row)
    row.pop("seconds", None)
    return json.dumps(row, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline", required=True)
    ap.add_argument("--baseline-sha256", required=True)
    ap.add_argument("--baseline-source-sha", required=True)
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--candidate-sha256", required=True)
    ap.add_argument("--candidate-source-sha", required=True)
    ap.add_argument("--output", required=True)
    ap.add_argument("--cpu", type=int, default=0)
    a = ap.parse_args()
    out = Path(a.output)
    out.mkdir(parents=True, exist_ok=False)
    bins = {"baseline": str(Path(a.baseline).resolve()),
            "candidate": str(Path(a.candidate).resolve())}
    expected = {"baseline": a.baseline_sha256, "candidate": a.candidate_sha256}
    for name, path in bins.items():
        if not Path(path).is_file():
            raise SystemExit(f"missing {name} binary: {path}")
        actual = sha(path)
        if actual != expected[name]:
            raise SystemExit(f"{name} binary SHA256 mismatch: {actual}")
    env = os.environ.copy()
    env.update({"RAYON_NUM_THREADS": "1", "OMP_NUM_THREADS": "1",
                "OPENBLAS_NUM_THREADS": "1", "MKL_NUM_THREADS": "1"})
    started = time.monotonic()
    attempts = []
    failures = []
    timing = {p: {k: [] for k in bins} for p in POLICIES}

    def run(name, argv, parse_json=True):
        remaining = TOTAL_BUDGET - (time.monotonic() - started)
        rec = {"name": name, "argv": argv, "timeout_seconds": min(CHILD_TIMEOUT, max(0, remaining)),
               "cpu": a.cpu}
        if remaining <= 0:
            rec.update(status="FAILED_BUDGET", exit_code=None, wall_seconds=0.0)
            put(out / (name + ".stdout"), b"")
            put(out / (name + ".stderr"), b"cumulative 600-second budget exhausted\n")
            rec["stdout_path"] = str(out / (name + ".stdout"))
            rec["stderr_path"] = str(out / (name + ".stderr"))
            attempts.append(rec)
            failures.append({"kind": "FAILED_BUDGET", "attempt": name})
            return rec, None
        preexec = None
        if hasattr(os, "sched_setaffinity"):
            def pin(): os.sched_setaffinity(0, {a.cpu})
            preexec = pin
        launch_started = time.monotonic()
        try:
            p = subprocess.run(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               timeout=min(CHILD_TIMEOUT, remaining), env=env,
                               preexec_fn=preexec, check=False)
            stdout, stderr = p.stdout, p.stderr
            rec.update(status="EXITED", exit_code=p.returncode)
        except subprocess.TimeoutExpired as e:
            stdout, stderr = e.stdout or b"", e.stderr or b""
            rec.update(status="TIMEOUT", exit_code=None)
        except Exception as e:
            stdout, stderr = b"", repr(e).encode()
            rec.update(status="LAUNCH_ERROR", exit_code=None, error=repr(e))
        rec["wall_seconds"] = time.monotonic() - launch_started
        rec["stdout_path"] = str(out / (name + ".stdout"))
        rec["stderr_path"] = str(out / (name + ".stderr"))
        put(rec["stdout_path"], stdout)
        put(rec["stderr_path"], stderr)
        parsed = None
        if rec["status"] == "EXITED" and rec["exit_code"] == 0 and parse_json:
            try:
                parsed = json.loads(stdout)
            except Exception as e:
                rec.update(status="INVALID_JSON", parse_error=repr(e))
        attempts.append(rec)
        if rec["status"] != "EXITED" or rec["exit_code"] != 0:
            failures.append({"kind": rec["status"] if rec["status"] != "EXITED" else "PROCESS_EXIT_NONZERO", "attempt": name})
        elif parse_json and parsed is None:
            failures.append({"kind": "INVALID_JSON", "attempt": name})
        return rec, parsed

    def invoke(binary, tag, policy, prof=None):
        if prof is None:
            argv = [binary, CASE, policy, str(REPEATS)]
        else:
            vg, n, cg = prof
            argv = [vg, "--tool=callgrind", "--cache-sim=no", "--branch-sim=no",
                    "--callgrind-out-file=" + str(cg), binary, CASE, policy, str(n)]
        rec, parsed = run(tag, argv)
        if prof is None and parsed is not None:
            rows = parsed.get("rows", [])
            if len(rows) != 1 or rows[0].get("case") != CASE or rows[0].get("policy") != policy:
                failures.append({"kind": "OUTPUT_CONTRACT_FAILURE", "attempt": tag,
                                 "row_count": len(rows)})
                return rec, None
            row = rows[0]
            if row.get("repeats") != REPEATS or not row.get("pass"):
                failures.append({"kind": "SOLVER_STATUS_FAILURE", "attempt": tag,
                                 "pass": row.get("pass"), "repeats": row.get("repeats")})
            return rec, row
        return rec, parsed

    pairs = []
    for policy in POLICIES:
        warm = {}
        for arm in bins:
            rec, row = invoke(bins[arm], f"warmup-{policy}-{arm}", policy)
            warm[arm] = row
        if all(warm.values()) and canon(warm["baseline"]) != canon(warm["candidate"]):
            failures.append({"kind": "EXACT_PARITY_FAILURE", "phase": "warmup", "policy": policy})
        for block in range(1, 6):
            order = (("baseline", "candidate", "candidate", "baseline") if block % 2
                     else ("candidate", "baseline", "baseline", "candidate"))
            block_rows = {arm: [] for arm in bins}
            for pos, arm in enumerate(order):
                tag = f"timing-{policy}-block{block}-{pos}-{arm}"
                rec, row = invoke(bins[arm], tag, policy)
                if row is not None:
                    timing[policy][arm].append(row["seconds"])
                    block_rows[arm].append(row)
            if all(len(block_rows[k]) == 2 for k in bins):
                for j in range(2):
                    if canon(block_rows["baseline"][j]) != canon(block_rows["candidate"][j]):
                        failures.append({"kind": "EXACT_PARITY_FAILURE", "policy": policy,
                                         "block": block, "replicate": j})
                pairs.append({"policy": policy, "block": block, "order": list(order),
                              "baseline_mean_seconds": statistics.mean(r["seconds"] for r in block_rows["baseline"]),
                              "candidate_mean_seconds": statistics.mean(r["seconds"] for r in block_rows["candidate"])})

    profile = {"status": "NOT_RUN"}
    vg, annotate = shutil.which("valgrind"), shutil.which("callgrind_annotate")
    if vg and annotate:
        profile = {"status": "RUNNING", "levels": [4, 12], "runs": []}
        for policy in POLICIES:
            for arm, binary in bins.items():
                for n in (4, 12):
                    name = f"callgrind-{policy}-{arm}-r{n}"
                    cg = out / (name + ".callgrind")
                    rec, _ = invoke(binary, name, policy, (vg, n, cg))
                    if rec.get("exit_code") == 0 and cg.exists():
                        for kind, inclusive in (("inclusive", "yes"), ("self", "no")):
                            aname = f"annotate-{kind}-{policy}-{arm}-r{n}"
                            arec, _ = run(aname, [annotate, f"--inclusive={inclusive}",
                                "--threshold=99.9", "--auto=no", str(cg)], parse_json=False)
                    profile["runs"].append({"name": name, "binary": arm, "policy": policy,
                                            "repeats": n, "callgrind": str(cg), "receipt": rec})
        profile["instruction_slopes"] = {}
        for policy in POLICIES:
            for arm in bins:
                points = []
                for n in (4, 12):
                    cg = out / f"callgrind-{policy}-{arm}-r{n}.callgrind"
                    points.append((n, callgrind_summary(cg) if cg.exists() else None))
                slope = None
                if all(x and x["summary"] and x["events"] for _, x in points):
                    if points[0][1]["events"] == points[1][1]["events"] and "Ir" in points[0][1]["events"]:
                        idx = points[0][1]["events"].index("Ir")
                        slope = (points[1][1]["summary"][idx] - points[0][1]["summary"][idx]) / 8
                profile["instruction_slopes"][f"{policy}/{arm}"] = {
                    "levels": points, "Ir_per_repeat": slope,
                    "scope": "includes process startup and JSON/output work; not solver-only"}
        profile["status"] = "COMPLETE_WITH_FAILURES" if failures else "COMPLETE"
    else:
        profile["reason"] = "valgrind or callgrind_annotate unavailable"
        profile["valgrind"] = vg
        profile["callgrind_annotate"] = annotate

    summary = {"schema": "reentrant-control-v1", "case": CASE, "policies": list(POLICIES),
      "repeats_per_process": REPEATS, "warmup_per_policy_arm": 1, "timed_blocks": 5,
      "order_rule": "odd ABBA, even BAAB", "process_timeout_seconds": CHILD_TIMEOUT,
      "cumulative_budget_seconds": TOTAL_BUDGET, "timing_scope": "rows[0].seconds; complete repeated solve interval",
      "external_work_lock": "caller-owned native benchmark flock", "source_sha": {
        "baseline": a.baseline_source_sha, "candidate": a.candidate_source_sha},
      "binary_sha256": {k: sha(v) for k, v in bins.items()}, "attempts": attempts,
      "paired_blocks": pairs, "profile": profile, "failures": failures,
      "status": "FAIL" if failures else "COMPLETE"}
    put(out / "summary.json", (json.dumps(summary, indent=2, sort_keys=True) + "\n").encode())
    print(json.dumps({"status": summary["status"], "attempts": len(attempts),
                      "failures": len(failures), "output": str(out)}))
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
