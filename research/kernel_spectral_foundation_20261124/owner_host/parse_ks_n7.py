#!/usr/bin/env python3
"""Parser of the KS-N7 owner-host timing cell (owner_host/KS_N7_PROTOCOL.json).

Reads only owner_host/logs/ks_n7/ and the protocol; writes results/KS_N7_OWNER_HOST.jsonl (one line
per process) and results/KS_N7_OWNER_HOST.json (ratios, decidability, verdict). No number typed in.

    python3 -I research/kernel_spectral_foundation_20261124/owner_host/parse_ks_n7.py
"""
import json
import pathlib
import re
import statistics
import sys

HERE = pathlib.Path(__file__).resolve().parent
UNIT = HERE.parent
LOGS = HERE / "logs" / "ks_n7"
PROTOCOL = json.loads((HERE / "KS_N7_PROTOCOL.json").read_text())
EVENTS = ("instructions:u", "cycles:u", "task-clock")
TAG = re.compile(r"r(\d)_([0-9x]+)_(complex|paired)_s(\d+)")


def perf_counts(path):
    counts = {}
    for line in path.read_text().splitlines():
        fields = line.split(",")
        if len(fields) >= 3 and fields[2] in EVENTS:
            counts[fields[2]] = None if fields[0].startswith("<") else float(fields[0])
    return counts


def main():
    rows, problems = [], []
    for meta_path in sorted(LOGS.glob("r*.meta.json")):
        tag = meta_path.name[: -len(".meta.json")]
        m = TAG.fullmatch(tag)
        if not m:
            problems.append(f"unexpected tag {tag}")
            continue
        text = (LOGS / f"{tag}.out.txt").read_text()
        found = re.search(r"KS_N7_MEASURE (\{.*\})", text)
        exit_code = int((LOGS / f"{tag}.exit").read_text().strip())
        row = {"tag": tag, "round": int(m[1]), "grid": m[2], "arm": m[3], "solves": int(m[4]), "exit": exit_code,
               "meta": json.loads(meta_path.read_text()), "counts": perf_counts(LOGS / f"{tag}.perf.csv"),
               "measure": json.loads(found[1]) if found else None}
        if exit_code != 0 or row["measure"] is None:
            problems.append(f"{tag}: exit {exit_code}, measure line {'found' if found else 'missing'}")
        rows.append(row)
    index = {(r["round"], r["grid"], r["arm"], r["solves"]): r for r in rows}
    worst_backward = max((r["measure"]["max_backward_error"] for r in rows if r["measure"]), default=None)
    cells = {}
    for spec in PROTOCOL["cells"]:
        grid, n = spec["grid"], spec["N"]
        per = {"complex": [], "paired": []}
        loads = []
        for rnd in range(1, PROTOCOL["rounds"] + 1):
            for arm in per:
                one, many = index.get((rnd, grid, arm, 1)), index.get((rnd, grid, arm, 1 + n))
                if one is None or many is None or many["measure"] is None:
                    problems.append(f"missing {grid} {arm} round {rnd}")
                    continue
                loads += [one["meta"]["load1_before"], many["meta"]["load1_before"]]
                per[arm].append({
                    "round": rnd, "median_s": many["measure"]["median_s"],
                    "instructions": (many["counts"]["instructions:u"] - one["counts"]["instructions:u"]) / n,
                    "cycles": (many["counts"]["cycles:u"] - one["counts"]["cycles:u"]) / n,
                })
        entry = {"grid": grid, "N": n, "deciding": spec["deciding"], "per_round": per,
                 "max_load1": max(loads) if loads else None}
        if per["complex"] and per["paired"] and len(per["complex"]) == len(per["paired"]):
            med = {arm: statistics.median(x["median_s"] for x in per[arm]) for arm in per}
            entry["wall_ratio"] = med["paired"] / med["complex"]
            entry["wall_ratio_per_round"] = [p["median_s"] / c["median_s"] for c, p in zip(per["complex"], per["paired"])]
            entry["instructions_ratio"] = (statistics.median(x["instructions"] for x in per["paired"])
                                           / statistics.median(x["instructions"] for x in per["complex"]))
            entry["cycles_ratio"] = (statistics.median(x["cycles"] for x in per["paired"])
                                     / statistics.median(x["cycles"] for x in per["complex"]))
            ratio = entry["wall_ratio"]
            entry["decidable"] = entry["max_load1"] <= 3.0 or ratio <= 0.3 or ratio >= 1.2
        cells[grid] = entry
    deciding = cells.get("256x256", {})
    if problems or worst_backward is None or worst_backward > 1e-13:
        verdict = "INVALID"
    elif not deciding.get("decidable"):
        verdict = "NOT_DECIDABLE"
    elif deciding["wall_ratio"] <= 0.6:
        verdict = "PASS"
    elif deciding["wall_ratio"] >= 1.0:
        verdict = "KILL"
    else:
        verdict = "FAIL"
    report = {"schema": "vigilode.ks-n7-results.v1", "task": "KS-N7 owner-host timing cell",
              "protocol": "research/kernel_spectral_foundation_20261124/owner_host/KS_N7_PROTOCOL.json",
              "environment": json.loads((LOGS / "env.json").read_text()),
              "end": json.loads((LOGS / "end.json").read_text()) if (LOGS / "end.json").exists() else None,
              "cells": cells, "max_backward_error": worst_backward, "problems": problems, "verdict": verdict,
              "fallback": PROTOCOL["plan"]["fallback"] if verdict in ("KILL", "FAIL") else None}
    out = UNIT / "results"
    with open(out / "KS_N7_OWNER_HOST.jsonl", "w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=True, allow_nan=False) + "\n")
    (out / "KS_N7_OWNER_HOST.json").write_text(json.dumps(report, indent=1, allow_nan=False) + "\n")
    for grid, e in cells.items():
        print(f"{grid}: wall ratio {e.get('wall_ratio')} per round {e.get('wall_ratio_per_round')} "
              f"instr {e.get('instructions_ratio')} cycles {e.get('cycles_ratio')} max load {e.get('max_load1')} "
              f"decidable {e.get('decidable')}")
    print(f"max backward error {worst_backward}; problems {problems}; verdict {verdict}")
    return 0 if not problems else 1


if __name__ == "__main__":
    sys.exit(main())
