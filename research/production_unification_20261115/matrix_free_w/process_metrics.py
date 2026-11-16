#!/usr/bin/env python3
"""Run c7_single_arm_process_metrics once per (n, arm, repetition) in a fresh process.

Each process integrates one arm only, so VmHWM is that arm's peak (plus the test harness
baseline, recorded as vm_hwm_kb_before).  CPU ticks use CLK_TCK from getconf.  Output:
logs/u04_process_metrics.jsonl and process_metrics_summary.json.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
TICK = int(subprocess.check_output(["getconf", "CLK_TCK"]).strip())
ARMS = ["dense", "mf_independent_seq", "mf_independent_r10_par6", "mf_block"]
NS = [50, 200, 400]
REPS = 3


def main():
    out = HERE / "logs/u04_process_metrics.jsonl"
    rows = []
    with out.open("w") as fh:
        for rep in range(REPS):
            for n in NS:
                for k, arm in enumerate(ARMS):
                    arm = ARMS[(k + rep) % len(ARMS)]
                    env = dict(os.environ, U04_N=str(n), U04_ARM=arm)
                    cmd = ["cargo", "+1.94.1", "test", "--locked", "--offline", "-q", "-p",
                           "rodas5p-integrators", "--release", "--test",
                           "ppsw1_linear_backend_contracts", "--", "--ignored", "--exact",
                           "c7_single_arm_process_metrics", "--nocapture"]
                    p = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True)
                    line = next((l for l in p.stdout.splitlines() if "QUAL {" in l), None)
                    if line is None:
                        row = {"n": n, "arm": arm, "rep": rep, "exit": p.returncode,
                               "error": p.stderr[-2000:]}
                    else:
                        row = json.loads(line[line.index("QUAL ") + 5:])["data"]
                        row["rep"] = rep
                        row["exit"] = p.returncode
                        row["cpu_seconds"] = (row["cpu_user_ticks"] + row["cpu_system_ticks"]) / TICK
                    fh.write(json.dumps(row) + "\n")
                    fh.flush()
                    rows.append(row)
                    print(n, arm, rep, row.get("status"), row.get("wall_seconds"),
                          row.get("cpu_seconds"), row.get("vm_hwm_kb_after"), file=sys.stderr)
    summary = {}
    for r in rows:
        if "cpu_seconds" not in r:
            continue
        key = f'{r["n"]}/{r["arm"]}'
        s = summary.setdefault(key, {"n": r["n"], "arm": r["arm"], "status": set(), "wall": [],
                                     "cpu": [], "hwm_kb": [], "baseline_hwm_kb": []})
        s["status"].add(r["status"])
        s["wall"].append(r["wall_seconds"])
        s["cpu"].append(r["cpu_seconds"])
        s["hwm_kb"].append(r["vm_hwm_kb_after"])
        s["baseline_hwm_kb"].append(r["vm_hwm_kb_before"])
    med = lambda v: sorted(v)[len(v) // 2]
    final = [{"n": s["n"], "arm": s["arm"], "status": sorted(s["status"]), "repetitions": len(s["wall"]),
              "median_wall_s": med(s["wall"]), "median_cpu_s": med(s["cpu"]),
              "median_peak_rss_kb": med(s["hwm_kb"]), "median_baseline_rss_kb": med(s["baseline_hwm_kb"]),
              "timing_authority": "diagnostic_only (3 repetitions, shared host)"}
             for s in summary.values()]
    (HERE / "process_metrics_summary.json").write_text(json.dumps(final, indent=1) + "\n")


if __name__ == "__main__":
    main()
