#!/usr/bin/env python3
"""Light single-process benchmark of selected arms (diagnostic timing only).

One process per (case, arm); 7 batches of >= 50 ms after calibration.
Arms are the adapter's selection plus one or two relevant direct routes,
not an all-method sweep.
"""
import json
import subprocess
import sys
from pathlib import Path

CELLS = {
    "rotation": ["adaptive", "evidence", "dopri"],
    "decay": ["adaptive", "dopri"],
    "case5": ["adaptive", "evidence", "rodas", "rodas-dense"],
    "stiff-forced": ["adaptive", "rodas"],
    "relaxation": ["adaptive", "adaptive-sentinel", "rodas"],
    "transition-39": ["adaptive", "adaptive-sentinel", "rodas"],
    "transition-79": ["adaptive", "rodas"],
    "gyro-tight": ["adaptive", "dopri", "rodas"],
    "gyro-default": ["adaptive", "dopri"],
    "stale-return": ["adaptive", "adaptive-sentinel", "dopri", "rodas"],
}


def main():
    binary, out = Path(sys.argv[1]), Path(sys.argv[2])
    rows = []
    for case, arms in CELLS.items():
        for arm in arms:
            r = subprocess.run(["taskset", "-c", "11", str(binary), "bench", case, arm],
                               capture_output=True, text=True, timeout=600,
                               env={"RAYON_NUM_THREADS": "1", "PATH": "/usr/bin:/bin"})
            row = json.loads(r.stdout) if r.returncode == 0 else {"case": case, "arm": arm,
                                                                   "error": r.stderr[-400:]}
            rows.append(row)
            print(case, arm, row.get("ns_per_solve_median"), row.get("allocations_per_solve"), flush=True)
    out.write_text(json.dumps({"schema": "vigilode.adapter-light-bench-set.v1",
                               "timing_authority": "diagnostic_only", "rows": rows}, indent=1) + "\n")


if __name__ == "__main__":
    main()
