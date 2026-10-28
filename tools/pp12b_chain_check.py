#!/usr/bin/env python3
"""Gate of research node research/pp12b_chain_symmetrizer_20261004 (PP12b).

G1: every bounded certificate (all metrics) encloses ||exp(tau A) v - candidate||_2
(50 digits). G2: mu_up >= lambda_max((B + B^T)/2) - 1e-90 (1 + |lambda_max|) with 100-digit
eigenvalues, B = D A D^-1 from the exported metric. G3: every F2 and holdout (H-*) case:
the smallest bounded error_upper <= 1e-8 ||exp(tau A) v||.
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
    data = json.loads(args.cases.read_text())
    g1 = g2 = g3 = True
    rows = []
    for case in data["cases"]:
        n = len(case["v"])
        mp.mp.dps = 50
        a = mp.matrix([[unhex(x) for x in r] for r in case["a"]])
        v = mp.matrix([unhex(x) for x in case["v"]])
        exact = mp.expm(a * unhex(case["tau"])) * v
        norm = mp.sqrt(mp.fsum(exact[i] ** 2 for i in range(n)))
        row = {"group": case["group"], "label": case["label"], "exact_norm": mp.nstr(norm, 8), "certificates": []}
        best = None
        for c in case["certificates"]:
            mp.mp.dps = 100
            d = [unhex(x) for x in c["metric_d"]]
            sym = mp.matrix(n, n)
            for i in range(n):
                for j in range(n):
                    sym[i, j] = (d[i] * a[i, j] / d[j] + d[j] * a[j, i] / d[i]) / 2
            lam = max(mp.eigsy(sym)[0])
            mu = unhex(c["mu_up"])
            ok2 = mu >= lam - mp.mpf("1e-90") * (1 + abs(lam))
            g2 &= bool(ok2)
            mp.mp.dps = 50
            entry = {"metric": c["metric"], "mu_up": mp.nstr(mu, 10), "lambda_max": mp.nstr(lam, 10),
                     "mu_source": c["mu_source"], "steps": c["steps"], "status": c["status"],
                     "transport": mp.nstr(unhex(c["transport"]), 6), "mu_encloses": bool(ok2)}
            if c["status"] == "bounded":
                x = [unhex(h) for h in c["candidate"]]
                err = mp.sqrt(mp.fsum((x[i] - exact[i]) ** 2 for i in range(n)))
                upper = unhex(c["error_upper"])
                inside = err <= upper
                g1 &= bool(inside)
                entry.update({"true_error": mp.nstr(err, 6), "error_upper": mp.nstr(upper, 6),
                              "relative_upper": mp.nstr(upper / norm, 6), "encloses": bool(inside)})
                if best is None or upper < best[0]:
                    best = (upper, c["metric"])
            row["certificates"].append(entry)
        if case["group"] == "F2" or case["group"].startswith("H-"):
            ok3 = best is not None and best[0] <= mp.mpf("1e-8") * norm
            g3 &= bool(ok3)
            row["stiff_threshold_met"] = bool(ok3)
        row["best"] = {"metric": best[1], "relative_upper": mp.nstr(best[0] / norm, 6)} if best else None
        rows.append(row)
        print(row["group"], row["label"], row["best"])
    result = {"schema": "vigilode-pp12b-check-v1", "rows": rows,
              "gate": {"g1_enclosure": g1, "g2_mu_encloses": g2, "g3_stiff_usefulness": g3}}
    result["verdict"] = "PASS" if (g1 and g2 and g3) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result["gate"]), result["verdict"])


if __name__ == "__main__":
    main()
