#!/usr/bin/env python3
"""50-digit check of research node research/rev02_nonnormal_stepping_20261003 (review DAG
node REV-02): stepped certificates (identity and Osborne metrics), INT-05 single-step
certificates for comparison, and the directed-exponential grid, against mpmath.expm and
mpmath.exp at 50 digits."""

from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path

import mpmath as mp

SCHEMA = "vigilode-rev02-nonnormal-stepping-v1"


def unhex(text: str):
    return mp.mpf(struct.unpack(">d", bytes.fromhex(text))[0])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    mp.mp.dps = 50
    data = json.loads(args.cases.read_text())
    rows = []
    enclosure = True
    stiff_ok = True
    auto_metric_ok = None
    for case in data["cases"]:
        a = mp.matrix([[unhex(x) for x in row] for row in case["a"]])
        v = mp.matrix([unhex(x) for x in case["v"]])
        exact = mp.expm(a * unhex(case["tau"])) * v
        norm = mp.sqrt(mp.fsum(exact[i] ** 2 for i in range(len(case["v"]))))
        row = {"group": case["group"], "label": case["label"], "chosen": case["chosen"], "exact_norm": mp.nstr(norm, 8)}
        uppers = {}
        for key in ("stepped_identity", "stepped_osborne", "single_step_identity_m30", "single_step_osborne_m30"):
            block = case[key]
            cert = block["certificate"] if "certificate" in block else block
            status = cert["status"]
            x = [unhex(h) for h in cert["candidate"]]
            if status == "bounded":
                err = mp.sqrt(mp.fsum((x[i] - exact[i]) ** 2 for i in range(len(x))))
                upper = mp.mpf(cert["error_upper"])
                inside = err <= upper
                if key.startswith("stepped"):
                    enclosure &= bool(inside)
                uppers[key] = upper
                row[key] = {"status": status, "steps": block.get("steps"), "true_error": mp.nstr(err, 6),
                            "error_upper": cert["error_upper"], "relative_upper": mp.nstr(upper / norm, 6),
                            "upper_over_true": mp.nstr(upper / err, 6) if err > 0 else None, "encloses": bool(inside)}
            else:
                row[key] = {"status": status, "steps": block.get("steps")}
        best = min([uppers[k] for k in ("stepped_identity", "stepped_osborne") if k in uppers], default=None)
        row["auto_relative_upper"] = mp.nstr(best / norm, 6) if best is not None else None
        if case["group"] == "F2":
            stiff_ok &= best is not None and best <= mp.mpf("1e-8") * norm
        if case["label"] == "vig-a02-k46":
            o = uppers.get("stepped_osborne")
            auto_metric_ok = o is not None and o <= mp.mpf("1e-10") * norm
        rows.append(row)
    grid_ok = True
    for g in data["exp_grid"]:
        x = unhex(g["x"])
        e = mp.exp(x)
        grid_ok &= bool(unhex(g["lo"]) <= e <= unhex(g["hi"]))
    gate = {"1_enclosure": bool(enclosure), "2_directed_exponential": bool(grid_ok),
            "3_stiff_usefulness": bool(stiff_ok), "4_automatic_metric": bool(auto_metric_ok),
            "5_contracts": "contract tests rev02_nonnormal_contracts"}
    verdict = "PASS" if all(v is True for k, v in gate.items() if not k.startswith("5")) else "FAIL"
    report = {"schema": SCHEMA, "mpmath": mp.__version__, "dps": mp.mp.dps, "rows": rows,
              "exp_grid_points": len(data["exp_grid"]), "gate": gate, "verdict": verdict}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"gate": gate, "verdict": verdict}))


if __name__ == "__main__":
    main()
