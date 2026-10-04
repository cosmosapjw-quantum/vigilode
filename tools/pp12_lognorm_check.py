#!/usr/bin/env python3
"""Gate of research node research/pp12_lognorm_decay_20261004 (RVJ DAG node PP12).

50 digits. G1: mu_up >= lambda_max((B + B^T)/2) for B = D A D^-1 from the exact binary64
A and the exported metric (identity or Osborne), for both certificates of every case.
G2: every bounded certificate encloses ||exp(tau A) v - candidate||_2. G3: every F2 case's
chosen bound <= 1e-8 ||exp(tau A) v|| (REV-02 item 3 threshold, unchanged).
"""

import argparse
import json
import struct
from pathlib import Path

import mpmath as mp


def unhex(t):
    return mp.mpf(struct.unpack(">d", bytes.fromhex(t))[0])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    mp.mp.dps = 50
    data = json.loads(args.cases.read_text())
    g1 = g2 = g3 = True
    rows = []
    for case in data["cases"]:
        n = len(case["v"])
        a = mp.matrix([[unhex(x) for x in r] for r in case["a"]])
        v = mp.matrix([unhex(x) for x in case["v"]])
        exact = mp.expm(a * unhex(case["tau"])) * v
        norm = mp.sqrt(mp.fsum(exact[i] ** 2 for i in range(n)))
        row = {"group": case["group"], "label": case["label"], "exact_norm": mp.nstr(norm, 8)}
        for key in ("chosen", "other"):
            c = case[key]
            d = [mp.mpf(1)] * n if c["metric"] == "identity" else [unhex(x) for x in case["osborne"]]
            sym = mp.matrix(n, n)
            for i in range(n):
                for j in range(n):
                    sym[i, j] = (d[i] * a[i, j] / d[j] + d[j] * a[j, i] / d[i]) / 2
            lam = max(mp.eigsy(sym)[0])
            mu = unhex(c["mu_up"])
            ok1 = mu >= lam
            g1 &= bool(ok1)
            entry = {"metric": c["metric"], "mu_up": mp.nstr(mu, 8), "lambda_max": mp.nstr(lam, 8),
                     "mu_source": c["mu_source"], "gershgorin_re_hi": mp.nstr(unhex(c["gershgorin_re_hi"]), 8),
                     "steps": c["steps"], "status": c["status"], "mu_encloses": bool(ok1)}
            if c["status"] == "bounded":
                x = [unhex(h) for h in c["candidate"]]
                err = mp.sqrt(mp.fsum((x[i] - exact[i]) ** 2 for i in range(n)))
                upper = unhex(c["error_upper"])
                inside = err <= upper
                g2 &= bool(inside)
                entry.update({"true_error": mp.nstr(err, 6), "error_upper": mp.nstr(upper, 6),
                              "relative_upper": mp.nstr(upper / norm, 6),
                              "upper_over_true": mp.nstr(upper / err, 6) if err > 0 else None,
                              "encloses": bool(inside)})
                if key == "chosen" and case["group"] == "F2":
                    ok3 = upper <= mp.mpf("1e-8") * norm
                    g3 &= bool(ok3)
                    entry["stiff_threshold_met"] = bool(ok3)
            elif key == "chosen" and case["group"] == "F2":
                g3 = False
            row[key] = entry
        rows.append(row)
    result = {"schema": "vigilode-pp12-check-v1", "dps": 50, "rows": rows,
              "gate": {"g1_mu_encloses": g1, "g2_enclosure": g2, "g3_stiff_usefulness": g3}}
    result["verdict"] = "PASS" if (g1 and g2 and g3) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    for r in rows:
        c = r["chosen"]
        print(r["group"], r["label"], c["metric"], c["mu_up"], c["lambda_max"], c.get("relative_upper"), c.get("upper_over_true"))
    print(json.dumps(result["gate"]), result["verdict"])


if __name__ == "__main__":
    main()
