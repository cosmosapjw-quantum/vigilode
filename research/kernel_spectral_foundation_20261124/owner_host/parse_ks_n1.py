#!/usr/bin/env python3
"""Parser of the KS-N1 owner-host series (owner_host/KS_N1_PROTOCOL.json).

Reads only owner_host/logs/ks_n1/ and the protocol. Writes
results/KS_N1_OWNER_HOST.jsonl (one line per measured process) and
results/KS_N1_OWNER_HOST.json (per-solve counts, ratios, identity, quiet rule, verdict).
No number is typed in; every value comes from the logs.

    python3 -I research/kernel_spectral_foundation_20261124/owner_host/parse_ks_n1.py
"""
import json
import pathlib
import re
import statistics
import sys

HERE = pathlib.Path(__file__).resolve().parent
UNIT = HERE.parent
LOGS = HERE / "logs" / "ks_n1"
PROTOCOL = json.loads((HERE / "KS_N1_PROTOCOL.json").read_text())
IDENTITY_FIELDS = ("rows", "n", "attempts", "accepted", "rejected", "jacobian_reuses", "lu", "work", "partial",
                   "success", "message", "counters", "internal_steps", "output_clipped_steps", "outputs",
                   "state_fnv1a")
EVENTS = ("instructions:u", "cycles:u", "task-clock")


def perf_counts(path):
    counts = {}
    for line in path.read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        fields = line.split(",")
        if len(fields) < 3 or fields[2] not in EVENTS:
            continue
        value = fields[0]
        counts[fields[2]] = None if value.startswith("<") else float(value)
    missing = [e for e in EVENTS if counts.get(e) is None]
    if missing:
        raise SystemExit(f"{path.name}: no count for {missing}")
    return counts


def main():
    cells = {c["id"]: c for c in PROTOCOL["cells"]}
    rows_out, by_key, problems = [], {}, []
    for meta_path in sorted(LOGS.glob("r*_*.meta.json")):
        tag = meta_path.name[: -len(".meta.json")]
        match = re.fullmatch(r"r(\d)_(n1e\d)_(indexed|fixed2)_s(\d+)", tag)
        if not match:
            problems.append(f"unexpected tag {tag}")
            continue
        rnd, cell, arm, solves = int(match[1]), match[2], match[3], int(match[4])
        meta = json.loads(meta_path.read_text())
        counts = perf_counts(LOGS / f"{tag}.perf.csv")
        probe = json.loads((LOGS / f"{tag}.out.json").read_text().strip())
        row = {"tag": tag, "round": rnd, "cell": cell, "arm": arm, "solves": solves,
               "instructions": counts["instructions:u"], "cycles": counts["cycles:u"],
               "task_clock_ms": counts["task-clock"], "load1_before": meta["load1_before"],
               "waited_s": meta["waited_s"], "start": meta["start"], "probe": probe}
        rows_out.append(row)
        by_key[(rnd, cell, arm, solves)] = row

    expected = PROTOCOL["rounds"] * len(cells) * 2 * 2
    if len(rows_out) != expected:
        problems.append(f"{len(rows_out)} measured processes, protocol expects {expected}")

    # Identity within each cell (driver id differs by design).
    identity = {}
    for cell in cells:
        lines = [r["probe"] for r in rows_out if r["cell"] == cell]
        reference = {k: lines[0].get(k) for k in IDENTITY_FIELDS} if lines else None
        mismatches = [
            {"tag": r["tag"], "field": k, "value": r["probe"].get(k), "reference": reference[k]}
            for r in rows_out if r["cell"] == cell for k in IDENTITY_FIELDS if r["probe"].get(k) != reference[k]
        ]
        drivers = sorted({(r["arm"], r["probe"].get("driver")) for r in rows_out if r["cell"] == cell})
        identity[cell] = {"lines": len(lines), "mismatches": mismatches, "drivers": drivers,
                          "attempts": reference and reference["attempts"],
                          "state_fnv1a": reference and reference["state_fnv1a"]}

    # Per-solve counts and ratios.
    summary = {}
    for cell, spec in cells.items():
        big = 1 + spec["N"]
        per = {"indexed": {"instructions": [], "cycles": []}, "fixed2": {"instructions": [], "cycles": []}}
        for rnd in range(1, PROTOCOL["rounds"] + 1):
            for arm in per:
                one, many = by_key.get((rnd, cell, arm, 1)), by_key.get((rnd, cell, arm, big))
                if one is None or many is None:
                    problems.append(f"{cell} round {rnd} {arm}: missing process")
                    continue
                for metric in ("instructions", "cycles"):
                    per[arm][metric].append((many[metric] - one[metric]) / spec["N"])
        entry = {"rows": spec["rows"], "n": spec["n"], "N": spec["N"], "per_solve": per}
        for metric in ("instructions", "cycles"):
            a, b = per["indexed"][metric], per["fixed2"][metric]
            if a and b and len(a) == len(b):
                entry[f"{metric}_ratio"] = statistics.median(b) / statistics.median(a)
                rounds = [y / x for x, y in zip(a, b)]
                entry[f"{metric}_ratio_per_round"] = rounds
                entry[f"{metric}_ratio_spread"] = max(rounds) / min(rounds) - 1.0
        summary[cell] = entry

    # Quiet rule.
    loud = [r["tag"] for r in rows_out if r["load1_before"] > 3.0]
    wide = [cell for cell, e in summary.items() if e.get("cycles_ratio_spread", 0.0) > 0.10]
    cycles_decidable = not loud and not wide

    # The non-ignored identity tests and the three diagnostic runs.
    identity_log = (LOGS / "identity_tests.log").read_text() if (LOGS / "identity_tests.log").exists() else ""
    results = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed", identity_log)
    identity_tests_ok = bool(results) and all(r[0] == "ok" and r[2] == "0" for r in results)
    diagnostics = []
    for k in (1, 2, 3):
        text_path = LOGS / f"diag_{k}.out.txt"
        if not text_path.exists():
            diagnostics.append({"run": k, "present": False})
            continue
        text = text_path.read_text()
        lines = [json.loads(l) for l in text.splitlines() if l.startswith('{"sl_n12"')]
        ok = "test result: ok. 1 passed" in text and text.rstrip().endswith("exit 0")
        counts = perf_counts(LOGS / f"diag_{k}.perf.csv")
        meta = json.loads((LOGS / f"diag_{k}.meta.json").read_text())
        diagnostics.append({"run": k, "present": True, "passed": ok, "wall_medians": lines,
                            "whole_process": counts, "load1_before": meta["load1_before"]})
    diagnostics_ok = all(d.get("passed") for d in diagnostics)

    probe_identity_ok = all(not v["mismatches"] and v["lines"] > 0 for v in identity.values())
    if problems or not probe_identity_ok or not identity_tests_ok or not diagnostics_ok:
        verdict = "INVALID"
    else:
        ins = [summary[c]["instructions_ratio"] for c in cells]
        cyc = [summary[c]["cycles_ratio"] for c in cells]
        if cycles_decidable:
            if all(i <= 0.60 for i in ins) and all(c <= 0.80 for c in cyc):
                verdict = "PASS"
            elif all(c > 0.95 for c in cyc):
                verdict = "KILL"
            else:
                verdict = "FAIL"
        else:
            verdict = "FAIL" if any(i > 0.60 for i in ins) else "NOT_DECIDABLE"

    env = json.loads((LOGS / "env.json").read_text())
    end = json.loads((LOGS / "end.json").read_text()) if (LOGS / "end.json").exists() else None
    report = {
        "schema": "vigilode.ks-n1-results.v1",
        "task": "KS-N1",
        "protocol": "research/kernel_spectral_foundation_20261124/owner_host/KS_N1_PROTOCOL.json",
        "environment": env,
        "end": end,
        "cells": summary,
        "identity": {"probe": identity, "probe_ok": probe_identity_ok, "identity_tests": results,
                     "identity_tests_ok": identity_tests_ok, "diagnostic_runs_ok": diagnostics_ok},
        "quiet_rule": {"processes_above_load_3": loud, "cells_with_cycles_spread_above_0_10": wide,
                       "cycles_decidable": cycles_decidable},
        "diagnostic_runs": diagnostics,
        "problems": problems,
        "verdict": verdict,
        "fallback": PROTOCOL["verdict"]["fallback_on_any_verdict_but_PASS"] if verdict != "PASS" else None,
    }
    out = UNIT / "results"
    with open(out / "KS_N1_OWNER_HOST.jsonl", "w", encoding="utf-8") as handle:
        for row in rows_out:
            handle.write(json.dumps(row, sort_keys=True, allow_nan=False) + "\n")
    (out / "KS_N1_OWNER_HOST.json").write_text(json.dumps(report, indent=1, allow_nan=False) + "\n")
    for cell, e in summary.items():
        print(f"{cell}: instructions ratio {e.get('instructions_ratio')}, cycles ratio {e.get('cycles_ratio')}, "
              f"cycles spread {e.get('cycles_ratio_spread')}")
    print(f"identity probe {probe_identity_ok}, tests {identity_tests_ok}, diagnostics {diagnostics_ok}; "
          f"cycles decidable {cycles_decidable}; verdict {verdict}")
    return 0 if not problems else 1


if __name__ == "__main__":
    sys.exit(main())
