#!/usr/bin/env python3
"""Alternate the base and candidate binaries on one arm (ABBA processes).

Diagnostic only: separate processes, not an in-process paired block. Records
are compared for identity (after the registered krylov_workspace id
normalisation); the time ratio is reported, never gated.
"""
import argparse
import json
import os
import statistics as st
import subprocess
from pathlib import Path

CELLS = [("case5", "rodas"), ("transition-39", "rodas"), ("transition-39", "adaptive"),
         ("rotation", "adaptive"), ("gyro-tight", "adaptive")]


def run(binary, case, arm, env):
    r = subprocess.run([str(binary), case, arm, arm, "1"], text=True, capture_output=True,
                       env=env, timeout=120, check=True)
    d = json.loads(r.stdout)
    return d, st.median(x["seconds_per_solve"] for x in d["timings"])


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("base", type=Path)
    p.add_argument("candidate", type=Path)
    p.add_argument("output", type=Path)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    env = os.environ.copy()
    env.update(RAYON_NUM_THREADS="1", OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1")
    rows = []
    for case, arm in CELLS:
        seconds = {"base": [], "candidate": []}
        records = {}
        for rep in range(3):
            for label in (["base", "candidate"] if rep % 2 == 0 else ["candidate", "base"]) * 1:
                d, s = run(a.base if label == "base" else a.candidate, case, arm, env)
                seconds[label].append(s)
                records[label] = d["initial_records"][arm]
            for label in (["candidate", "base"] if rep % 2 == 0 else ["base", "candidate"]):
                d, s = run(a.base if label == "base" else a.candidate, case, arm, env)
                seconds[label].append(s)
        norm = json.dumps(records["base"], sort_keys=True).replace(
            "krylov-ws-legacy-v1", "krylov-ws-ls-workspace-v1")
        rows.append({"case": case, "arm": arm, "seconds_per_solve": seconds,
                     "candidate_over_base_median": st.median(seconds["candidate"]) / st.median(seconds["base"]),
                     "ratio_range_of_process_medians": [min(seconds["candidate"]) / max(seconds["base"]),
                                                        max(seconds["candidate"]) / min(seconds["base"])],
                     "records_identical_after_krylov_id_normalisation":
                         norm == json.dumps(records["candidate"], sort_keys=True)})
        print(case, arm, round(rows[-1]["candidate_over_base_median"], 3),
              rows[-1]["records_identical_after_krylov_id_normalisation"], flush=True)
    (a.output / "CROSS_BINARY.json").write_text(json.dumps(
        {"schema": "vigilode.adapter-followup-cross-binary.v1", "timing_authority": "diagnostic_only",
         "design": "6 alternating processes per binary and cell, 1 block each", "rows": rows}, indent=1) + "\n")


if __name__ == "__main__":
    main()
