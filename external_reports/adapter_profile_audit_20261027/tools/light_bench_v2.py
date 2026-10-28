#!/usr/bin/env python3
"""Light single-process benchmark, harness v2 (diagnostic timing only).

Only the solver call is timed (summary built outside timing); allocations
are reported for the cold first call and a warm call. A cell whose batch
spread (max-min)/median exceeds 5 % is rerun up to twice; every attempt is
kept and the attempt with the smallest spread is marked selected.
"""
import json
import subprocess
import sys
from pathlib import Path

CELLS = {
    "rotation": ["adaptive", "evidence", "dopri"],
    "decay": ["adaptive", "dopri"],
    "case5": ["adaptive", "evidence", "rodas", "rodas-dense", "rodas-fd"],
    "stiff-forced": ["adaptive", "evidence", "rodas"],
    "relaxation": ["adaptive", "evidence", "adaptive-sentinel", "rodas"],
    "transition-39": ["adaptive", "evidence", "adaptive-sentinel", "rodas"],
    "transition-79": ["adaptive", "rodas"],
    "gyro-tight": ["adaptive", "dopri", "rodas", "rodas-dense", "rodas-fd"],
    "gyro-default": ["adaptive", "dopri"],
    "stale-return": ["adaptive", "adaptive-sentinel", "dopri", "rodas"],
}


def run(binary, case, arm):
    r = subprocess.run(["taskset", "-c", "11", str(binary), "bench", case, arm], capture_output=True,
                       text=True, timeout=900, env={"RAYON_NUM_THREADS": "1", "PATH": "/usr/bin:/bin"})
    if r.returncode != 0:
        return {"case": case, "arm": arm, "error": r.stderr[-400:]}
    return json.loads(r.stdout)


def main():
    binary, out = Path(sys.argv[1]), Path(sys.argv[2])
    rows = []
    for case, arms in CELLS.items():
        for arm in arms:
            attempts = [run(binary, case, arm)]
            while "error" not in attempts[-1] and attempts[-1]["batch_spread"] > 0.05 and len(attempts) < 3:
                attempts.append(run(binary, case, arm))
            best = min(attempts, key=lambda x: x.get("batch_spread", 9e9))
            row = dict(best, attempts=len(attempts),
                       spreads=[x.get("batch_spread") for x in attempts],
                       spread_ok=best.get("batch_spread", 9e9) <= 0.05)
            rows.append(row)
            print(case, arm, row.get("ns_per_solve_min"), row.get("allocations_warm"),
                  [round(x, 3) for x in row["spreads"] if x is not None], flush=True)
    out.write_text(json.dumps({"schema": "vigilode.adapter-light-bench-set.v2",
                               "timing_authority": "diagnostic_only", "rows": rows}, indent=1) + "\n")


if __name__ == "__main__":
    main()
