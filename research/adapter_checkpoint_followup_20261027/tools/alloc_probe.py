#!/usr/bin/env python3
"""Counted allocator events of one facade matrix-free RODAS solve, base vs candidate library.

Both binaries are the same trace example source; only the library differs.
The second (warm) solve of each process is used; per-attempt marginal is
(allocations at 40 attempts - at 20) / (attempt difference).
"""
import argparse
import json
import re
import subprocess
from pathlib import Path


def probe(binary, case, k):
    r = subprocess.run([str(binary), "repeat-rodas", case, "mf", str(k), "2"], text=True,
                       capture_output=True, check=True, timeout=120)
    rows = re.findall(r"attempts=(\d+) allocations=(\d+)", r.stderr)
    attempts, allocations = map(int, rows[1])
    return attempts, allocations


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("base", type=Path)
    p.add_argument("candidate", type=Path)
    p.add_argument("output", type=Path)
    a = p.parse_args()
    out = {"schema": "vigilode.adapter-followup-alloc.v1", "rows": []}
    for case in ["transition-39", "case5"]:
        row = {"case": case}
        for label, binary in [("base", a.base), ("candidate", a.candidate)]:
            pts = {k: probe(binary, case, k) for k in (10, 20, 40)}
            (a20, m20), (a40, m40) = pts[20], pts[40]
            row[label] = {"points": {str(k): {"attempts": v[0], "allocations": v[1]} for k, v in pts.items()},
                          "marginal_per_attempt": (m40 - m20) / (a40 - a20)}
        row["candidate_over_base_marginal"] = (row["candidate"]["marginal_per_attempt"]
                                               / row["base"]["marginal_per_attempt"])
        out["rows"].append(row)
        print(case, row["base"]["marginal_per_attempt"], row["candidate"]["marginal_per_attempt"], flush=True)
    a.output.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
