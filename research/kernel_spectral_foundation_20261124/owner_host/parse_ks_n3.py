#!/usr/bin/env python3
"""Parser of the KS-N3 owner-host series (owner_host/KS_N3_PROTOCOL.json).

Reads only owner_host/logs/ks_n3/ and the protocol. Writes results/KS_N3_OWNER_HOST.jsonl (one line
per measured process) and results/KS_N3_OWNER_HOST.json (per-solve values, ratios, decidability,
consistency, verdict). No number is typed in.

    python3 -I research/kernel_spectral_foundation_20261124/owner_host/parse_ks_n3.py
"""
import json
import pathlib
import re
import statistics
import sys

HERE = pathlib.Path(__file__).resolve().parent
UNIT = HERE.parent
LOGS = HERE / "logs" / "ks_n3"
PROTOCOL = json.loads((HERE / "KS_N3_PROTOCOL.json").read_text())
EVENTS = ("instructions:u", "cycles:u", "task-clock")
CONSISTENT = ("state_fnv1a", "relative_error", "attempts", "rhs_calls", "jvp_calls", "linear_iterations",
              "direct_solve_calls", "mass_drift", "success")
TAG = re.compile(r"r(\d)_(whole|stage)_(\d+x\d+)_(?:h([0-9.]+)_)?([a-z-]+)_s(\d+)")
H_TAGS = ("0.001", "0.01", "0.1")


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
        exit_code = int((LOGS / f"{tag}.exit").read_text().strip())
        out_text = (LOGS / f"{tag}.out.json").read_text().strip()
        row = {"tag": tag, "round": int(m[1]), "mode": m[2], "grid": m[3], "h": m[4], "arm": m[5],
               "solves": int(m[6]), "exit": exit_code, "meta": json.loads(meta_path.read_text()),
               "counts": perf_counts(LOGS / f"{tag}.perf.csv"),
               "probe": json.loads(out_text) if out_text.startswith("{") else None}
        if exit_code != 0 or row["probe"] is None:
            problems.append(f"{tag}: exit {exit_code}")
        rows.append(row)
    index = {(r["round"], r["mode"], r["grid"], r["h"], r["arm"], r["solves"]): r for r in rows}

    whole, stage = PROTOCOL["cells"]["whole"], PROTOCOL["cells"]["stage"]
    cells = {}
    for mode, spec in (("whole", whole), ("stage", stage)):
        hs = [None] if mode == "whole" else list(H_TAGS)
        for nt, np_ in spec["grids"]:
            grid = f"{nt}x{np_}"
            for h in hs:
                for arm in spec["arms"]:
                    n = spec["N"][arm][grid] if mode == "whole" else spec["N"][arm]
                    entry = {"mode": mode, "grid": grid, "h": h, "arm": arm, "N": n, "rounds": []}
                    for rnd in range(1, PROTOCOL["rounds"] + 1):
                        one = index.get((rnd, mode, grid, h, arm, 1))
                        many = index.get((rnd, mode, grid, h, arm, 1 + n))
                        if one is None or many is None:
                            problems.append(f"missing {mode} {grid} h={h} {arm} round {rnd}")
                            continue
                        probe = many["probe"] or {}
                        times = probe.get("times_s", [])
                        failed = probe.get("converged") is False
                        per = {"round": rnd, "failed": failed,
                               "load1": [one["meta"]["load1_before"], many["meta"]["load1_before"]]}
                        if not failed and one["counts"].get("instructions:u") and many["counts"].get("instructions:u"):
                            per["instructions"] = (many["counts"]["instructions:u"] - one["counts"]["instructions:u"]) / n
                            per["cycles"] = (many["counts"]["cycles:u"] - one["counts"]["cycles:u"]) / n
                            per["wall"] = statistics.median(times) if times else None
                        entry["rounds"].append(per)
                        for key in ("relative_error", "mass_drift", "attempts", "matvecs", "relative_residual",
                                    "state_fnv1a", "error"):
                            if key in probe:
                                entry.setdefault(key, probe[key])
                    good = [r for r in entry["rounds"] if "instructions" in r and r.get("wall") is not None]
                    if good and len(good) == len(entry["rounds"]):
                        for metric in ("instructions", "cycles", "wall"):
                            entry[metric] = statistics.median(r[metric] for r in good)
                    entry["max_load1"] = max((max(r["load1"]) for r in entry["rounds"]), default=None)
                    entry["failed"] = any(r["failed"] for r in entry["rounds"])
                    cells[(mode, grid, h, arm)] = entry

    # Consistency of whole cells: the solves are deterministic.
    mismatches = []
    for (mode, grid, h, arm) in cells:
        if mode != "whole":
            continue
        probes = [r["probe"] for r in rows if r["mode"] == "whole" and r["grid"] == grid and r["arm"] == arm
                  and r["probe"] and r["solves"] > 0]
        for key in CONSISTENT:
            values = {json.dumps(p.get(key)) for p in probes}
            if len(values) > 1:
                mismatches.append({"grid": grid, "arm": arm, "field": key, "values": sorted(values)})

    def decidable(a, b):
        ratio = a["wall"] / b["wall"]
        quiet = max(a["max_load1"], b["max_load1"]) <= 3.0
        return quiet or not (1 / 3 <= ratio <= 3), ratio

    grids = [f"{nt}x{np_}" for nt, np_ in whole["grids"]]
    acceptance, undecidable = {}, []
    for grid in grids:
        sep = cells[("whole", grid, None, "separable-verified")]
        ship = cells[("whole", grid, None, "shipped")]
        trusted = cells[("whole", grid, None, "separable-trusted")]
        ok_dec, wall_ratio = decidable(sep, ship)
        if not ok_dec:
            undecidable.append(f"whole {grid}")
        acceptance[grid] = {
            "instructions_ratio": sep["instructions"] / ship["instructions"],
            "cycles_ratio": sep["cycles"] / ship["cycles"],
            "wall_ratio": wall_ratio, "wall_decidable": ok_dec,
            "instructions_below": sep["instructions"] < ship["instructions"],
            "wall_below": sep["wall"] < ship["wall"],
            "error_separable": sep.get("relative_error"), "error_shipped": ship.get("relative_error"),
            "error_within_10x": sep["relative_error"] <= 10 * ship["relative_error"],
            "mass_drift_verified": sep.get("mass_drift"), "mass_drift_trusted": trusted.get("mass_drift"),
            "mass_drift_shipped": ship.get("mass_drift"),
            "mass_drift_ok": sep["mass_drift"] <= 1e-13 and trusted["mass_drift"] <= 1e-13,
            "trusted_instructions_ratio": trusted["instructions"] / ship["instructions"],
            "trusted_wall_ratio": trusted["wall"] / ship["wall"],
            "attempts": {"separable-verified": sep.get("attempts"), "shipped": ship.get("attempts")},
        }
    kill_per_grid, stage_ratios = {}, {}
    for grid in grids:
        slower = []
        for h in H_TAGS:
            sep = cells[("stage", grid, h, "separable")]
            gl = cells[("stage", grid, h, "gmres-longitude")]
            ok_dec, ratio = decidable(sep, gl)
            stage_ratios[f"{grid} h={h}"] = {"wall_ratio": ratio, "decidable": ok_dec,
                                            "instructions_ratio": sep["instructions"] / gl["instructions"],
                                            "separable_residual": sep.get("relative_residual"),
                                            "gmres_longitude_residual": gl.get("relative_residual")}
            slower.append(ok_dec and ratio > 1.0)
        kill_per_grid[grid] = all(slower)
    kill = all(kill_per_grid.values())

    accepted = all(a["instructions_below"] and a["wall_below"] and a["error_within_10x"] and a["mass_drift_ok"]
                   for a in acceptance.values())
    decidable_failure = any(
        (not a["instructions_below"]) or (a["wall_decidable"] and not a["wall_below"])
        or (not a["error_within_10x"]) or (not a["mass_drift_ok"]) for a in acceptance.values())
    if problems or mismatches:
        verdict = "INVALID"
    elif kill:
        verdict = "KILL"
    elif decidable_failure:
        verdict = "FAIL"
    elif undecidable:
        verdict = "NOT_DECIDABLE"
    elif accepted:
        verdict = "PASS"
    else:
        verdict = "FAIL"

    report = {
        "schema": "vigilode.ks-n3-results.v1",
        "task": "KS-N3",
        "protocol": "research/kernel_spectral_foundation_20261124/owner_host/KS_N3_PROTOCOL.json",
        "environment": json.loads((LOGS / "env.json").read_text()),
        "end": json.loads((LOGS / "end.json").read_text()) if (LOGS / "end.json").exists() else None,
        "cells": [dict(v, key=[k[0], k[1], k[2], k[3]]) for k, v in cells.items()],
        "acceptance_per_grid": acceptance,
        "stage_ratios": stage_ratios,
        "kill_per_grid": kill_per_grid,
        "undecidable_wall_comparisons": undecidable,
        "consistency_mismatches": mismatches,
        "problems": problems,
        "verdict": verdict,
        "fallback": PROTOCOL["plan"]["fallback"] if verdict != "PASS" else None,
    }
    out = UNIT / "results"
    with open(out / "KS_N3_OWNER_HOST.jsonl", "w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=True, allow_nan=False) + "\n")
    (out / "KS_N3_OWNER_HOST.json").write_text(json.dumps(report, indent=1, allow_nan=False) + "\n")
    for grid, a in acceptance.items():
        print(f"whole {grid}: instr {a['instructions_ratio']:.4g} wall {a['wall_ratio']:.4g} "
              f"(decidable {a['wall_decidable']}) err {a['error_separable']:.3g}/{a['error_shipped']:.3g} "
              f"drift {a['mass_drift_verified']:.2g}")
    for key, s in stage_ratios.items():
        print(f"stage {key}: separable/gmres-longitude wall {s['wall_ratio']:.4g} instr {s['instructions_ratio']:.4g}")
    print(f"problems {len(problems)}, mismatches {len(mismatches)}, undecidable {undecidable}; verdict {verdict}")
    return 0 if not problems else 1


if __name__ == "__main__":
    sys.exit(main())
